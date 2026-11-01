use super::*;
use anyhow::ensure;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Barrier;
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn independent_workers_distribute_jobs_and_preserve_retry_and_failure_outcomes()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let transaction = database.connection.begin().await?;
    let mut identifiers = Vec::new();
    for index in 0..16 {
        let identifier = Uuid::now_v7();
        let kind = match index {
            0 => "test.retry",
            1 => "test.permanent",
            _ => "test.success",
        };
        jobs::enqueue(
            &transaction,
            identifier,
            kind,
            &serde_json::json!({}),
            TraceContext::default(),
        )
        .await?;
        identifiers.push(identifier);
    }
    transaction.commit().await?;
    let first_connection = sea_orm::Database::connect(database.url.as_str()).await?;
    let second_connection = sea_orm::Database::connect(database.url.as_str()).await?;
    let shutdown = CancellationToken::new();
    let first_claims = Arc::new(Barrier::new(2));
    let executions = Arc::new(Mutex::new(BTreeMap::<Uuid, Vec<usize>>::new()));
    let settings = jobs::WorkerSettings {
        concurrency: 1,
        lease_seconds: 10,
        execution_seconds: 30,
        drain_seconds: 3,
        poll_milliseconds: 10,
    };
    let mut workers = Vec::new();
    for (worker_index, connection) in [&first_connection, &second_connection]
        .into_iter()
        .enumerate()
    {
        let first_claim = Arc::new(AtomicBool::new(true));
        let first_claims = first_claims.clone();
        let executions = executions.clone();
        workers.push(tokio::spawn(jobs::run(
            PostgresQueue::new(connection.clone()),
            settings,
            shutdown.clone(),
            move |claim| {
                let first_claim = first_claim.clone();
                let first_claims = first_claims.clone();
                let executions = executions.clone();
                async move {
                    if first_claim.swap(false, Ordering::SeqCst) {
                        first_claims.wait().await;
                    }
                    executions
                        .lock()
                        .unwrap()
                        .entry(claim.id)
                        .or_default()
                        .push(worker_index);
                    match claim.kind.as_str() {
                        "test.retry" if claim.attempts == 1 => Err(jobs::HandlerFailure {
                            code: "expected_retry",
                            retryable: true,
                        }),
                        "test.permanent" => Err(jobs::HandlerFailure {
                            code: "expected_permanent_failure",
                            retryable: false,
                        }),
                        _ => Ok(()),
                    }
                }
            },
        )));
    }
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            ensure!(
                workers.iter().all(|worker| !worker.is_finished()),
                "a worker exited unexpectedly"
            );
            let finished = job::Entity::find()
                .filter(
                    job::Column::State.is_in([jobs::JobState::Succeeded, jobs::JobState::Failed]),
                )
                .count(&database.connection)
                .await?;
            if finished == identifiers.len() as u64 {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    shutdown.cancel();
    for worker in workers {
        tokio::time::timeout(Duration::from_secs(5), worker).await???;
    }
    first_connection.close().await?;
    second_connection.close().await?;
    result??;
    {
        let executions = executions.lock().unwrap();
        assert_eq!(executions.len(), identifiers.len());
        assert_eq!(executions[&identifiers[0]].len(), 2);
        for identifier in &identifiers[1..] {
            assert_eq!(executions[identifier].len(), 1);
        }
        assert!(executions.values().flatten().any(|worker| *worker == 0));
        assert!(executions.values().flatten().any(|worker| *worker == 1));
    }
    let retry = job::Entity::find_by_id(identifiers[0])
        .one(&database.connection)
        .await?
        .unwrap();
    assert_eq!(retry.state, jobs::JobState::Succeeded);
    assert_eq!(retry.attempts, 2);
    let permanent = job::Entity::find_by_id(identifiers[1])
        .one(&database.connection)
        .await?
        .unwrap();
    assert_eq!(permanent.state, jobs::JobState::Failed);
    assert_eq!(
        permanent.last_error.as_deref(),
        Some("expected_permanent_failure")
    );
    assert_eq!(permanent.attempts, 1);
    assert_eq!(
        job_attempt::Entity::find()
            .count(&database.connection)
            .await?,
        17
    );
    database.finish().await
}
