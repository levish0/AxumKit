use super::*;
use anyhow::{Context, ensure};
use application::identity::entities::account;
use sea_orm::{ActiveModelTrait, Database, QuerySelect, Set};
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn workers_observe_business_changes_and_jobs_only_after_their_shared_commit()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let observer = Database::connect(database.url.as_str()).await?;
    let queue = PostgresQueue::new(observer.clone());
    let account_id = Uuid::now_v7();
    let job_id = Uuid::now_v7();
    let transaction = database.connection.begin().await?;
    account::ActiveModel {
        id: Set(account_id),
        email: Set(format!("{account_id}@example.test")),
        ..Default::default()
    }
    .insert(&transaction)
    .await?;
    jobs::enqueue(
        &transaction,
        job_id,
        "test.account_change",
        &serde_json::json!({"account_id": account_id}),
        TraceContext::default(),
    )
    .await?;
    ensure!(
        account::Entity::find_by_id(account_id)
            .one(&observer)
            .await?
            .is_none()
    );
    ensure!(queue.claim(30, 120).await?.is_none());
    transaction.commit().await?;
    let claim = queue
        .claim(30, 120)
        .await?
        .context("committed job missing")?;
    ensure!(claim.id == job_id);
    let original = account::Entity::find_by_id(account_id)
        .one(&observer)
        .await?
        .context("worker cannot see the committed business change")?;
    queue.finish(&claim, None).await?;

    let rolled_back_id = Uuid::now_v7();
    let transaction = database.connection.begin().await?;
    account::Entity::update_many()
        .col_expr(
            account::Column::SecurityVersion,
            Expr::value(original.security_version + 1),
        )
        .filter(account::Column::Id.eq(account_id))
        .exec(&transaction)
        .await?;
    jobs::enqueue(
        &transaction,
        rolled_back_id,
        "test.account_change",
        &serde_json::json!({"account_id": account_id}),
        TraceContext::default(),
    )
    .await?;
    ensure!(queue.claim(30, 120).await?.is_none());
    transaction.rollback().await?;
    ensure!(queue.claim(30, 120).await?.is_none());
    ensure!(
        job::Entity::find_by_id(rolled_back_id)
            .one(&observer)
            .await?
            .is_none()
    );
    ensure!(
        account::Entity::find_by_id(account_id)
            .one(&observer)
            .await?
            == Some(original)
    );
    observer.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn saturated_workers_release_database_locks_and_leave_new_jobs_unclaimed()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut expected = BTreeSet::new();
    for _ in 0..4 {
        expected.insert(queued_job(&database).await?);
    }
    let shutdown = CancellationToken::new();
    let gate = Arc::new(Semaphore::new(0));
    let (started, mut starts) = mpsc::unbounded_channel();
    let mut workers = tokio::task::JoinSet::new();
    let mut connections = Vec::new();
    for _ in 0..2 {
        let connection = Database::connect(database.url.as_str()).await?;
        connections.push(connection.clone());
        let gate = gate.clone();
        let started = started.clone();
        workers.spawn(jobs::run(
            PostgresQueue::new(connection),
            jobs::WorkerSettings {
                concurrency: 2,
                ..Default::default()
            },
            shutdown.clone(),
            move |claim| {
                let gate = gate.clone();
                let started = started.clone();
                async move {
                    started.send(claim.id).expect("start receiver remains open");
                    gate.acquire().await.expect("gate remains open").forget();
                    Ok(())
                }
            },
        ));
    }
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let mut observed = BTreeSet::new();
        for _ in 0..4 {
            ensure!(observed.insert(starts.recv().await.context("worker stopped")?));
        }
        ensure!(observed == expected);
        // A running handler must not retain the claim transaction's row lock.
        let transaction = database.connection.begin().await?;
        job::Entity::find()
            .filter(job::Column::Id.is_in(expected.iter().copied()))
            .lock_exclusive()
            .all(&transaction)
            .await?;
        for _ in 0..32 {
            let job_id = Uuid::now_v7();
            jobs::enqueue(
                &transaction,
                job_id,
                "test.execution",
                &serde_json::json!({}),
                TraceContext::default(),
            )
            .await?;
            expected.insert(job_id);
        }
        transaction.commit().await?;
        ensure!(
            job_attempt::Entity::find()
                .count(&database.connection)
                .await?
                == 4
        );
        ensure!(
            job::Entity::find()
                .filter(job::Column::State.eq(jobs::JobState::Pending))
                .count(&database.connection)
                .await?
                == 32
        );
        gate.add_permits(expected.len());
        for _ in 4..expected.len() {
            ensure!(
                observed.insert(starts.recv().await.context("worker stopped")?),
                "a job ran twice without a failure"
            );
        }
        loop {
            let succeeded = job::Entity::find()
                .filter(job::Column::State.eq(jobs::JobState::Succeeded))
                .count(&database.connection)
                .await?;
            if succeeded == expected.len() as u64 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        ensure!(observed == expected);
        ensure!(
            job_attempt::Entity::find()
                .count(&database.connection)
                .await?
                == expected.len() as u64
        );
        Ok::<(), anyhow::Error>(())
    })
    .await;
    shutdown.cancel();
    gate.add_permits(64);
    while let Some(stopped) = workers.join_next().await {
        stopped??;
    }
    for connection in connections {
        connection.close().await?;
    }
    let cleanup = database.finish().await;
    result??;
    cleanup
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn retention_drains_multiple_batches_without_removing_ready_running_or_recent_jobs()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let old = chrono::Utc::now() - chrono::Duration::days(91);
    let history = (0..250)
        .map(|_| job::ActiveModel {
            id: Set(Uuid::now_v7()),
            kind: Set("test.history".into()),
            queue: Set("application".into()),
            payload: Set(serde_json::json!({})),
            request_id: Set(Uuid::now_v7()),
            max_attempts: Set(5),
            state: Set(jobs::JobState::Succeeded),
            finished_at: Set(Some(old)),
            ..Default::default()
        })
        .collect::<Vec<_>>();
    job::Entity::insert_many(history)
        .exec(&database.connection)
        .await?;
    let running_id = queued_job(&database).await?;
    let running = queue
        .claim(30, 120)
        .await?
        .context("running fixture missing")?;
    ensure!(running.id == running_id);
    let recent_id = queued_job(&database).await?;
    let recent = queue
        .claim(30, 120)
        .await?
        .context("recent fixture missing")?;
    queue.finish(&recent, None).await?;
    let ready_id = queued_job(&database).await?;
    for expected in [100, 100, 50, 0] {
        ensure!(queue.cleanup_finished(30, 90).await? == expected);
    }
    ensure!(job::Entity::find().count(&database.connection).await? == 3);
    queue.heartbeat(&running, 30).await?;
    queue.finish(&running, None).await?;
    let ready = queue
        .claim(30, 120)
        .await?
        .context("ready job lost during cleanup")?;
    ensure!(ready.id == ready_id);
    queue.finish(&ready, None).await?;
    ensure!(
        job::Entity::find_by_id(recent_id)
            .one(&database.connection)
            .await?
            .is_some()
    );
    database.finish().await
}
