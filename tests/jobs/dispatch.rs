use super::*;
use anyhow::{Context, ensure};
use sea_orm::Database;
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Semaphore, mpsc};
use tokio_util::sync::CancellationToken;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn ready_jobs_fill_and_refill_worker_capacity_without_waiting_for_polling()
-> anyhow::Result<()> {
    check_worker_capacity(1).await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn independent_worker_pools_fill_and_refill_capacity_without_duplicate_jobs()
-> anyhow::Result<()> {
    for worker_count in [2, 4] {
        check_worker_capacity(worker_count).await?;
    }
    Ok(())
}

async fn check_worker_capacity(worker_count: usize) -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let concurrency = 3;
    let mut job_ids = BTreeSet::new();
    for _ in 0..(concurrency * worker_count + 5) {
        job_ids.insert(queued_job(&database).await?);
    }
    let shutdown = CancellationToken::new();
    let permits = Arc::new(Semaphore::new(0));
    let (started, mut starts) = mpsc::unbounded_channel();
    let active = Arc::new(
        (0..worker_count)
            .map(|_| AtomicUsize::new(0))
            .collect::<Vec<_>>(),
    );
    let peak_active = Arc::new(
        (0..worker_count)
            .map(|_| AtomicUsize::new(0))
            .collect::<Vec<_>>(),
    );
    let settings = jobs::WorkerSettings {
        concurrency,
        poll_milliseconds: 60_000,
        ..Default::default()
    };
    let mut connections = Vec::new();
    let mut workers = Vec::new();
    for worker_index in 0..worker_count {
        let connection = Database::connect(database.url.as_str()).await?;
        workers.push(tokio::spawn(jobs::run(
            PostgresQueue::new(connection.clone()),
            settings,
            shutdown.clone(),
            {
                let permits = permits.clone();
                let active = active.clone();
                let peak_active = peak_active.clone();
                let started = started.clone();
                move |claim| {
                    let permits = permits.clone();
                    let started = started.clone();
                    let active = active.clone();
                    let peak_active = peak_active.clone();
                    async move {
                        let running = active[worker_index].fetch_add(1, Ordering::SeqCst) + 1;
                        peak_active[worker_index].fetch_max(running, Ordering::SeqCst);
                        started.send(claim.id).expect("start receiver is open");
                        permits.acquire().await.expect("test gate is open").forget();
                        active[worker_index].fetch_sub(1, Ordering::SeqCst);
                        Ok(())
                    }
                }
            },
        )));
        connections.push(connection);
    }
    let total_capacity = settings.concurrency * worker_count;
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let mut executions = BTreeSet::new();
        for _ in 0..total_capacity {
            ensure!(executions.insert(starts.recv().await.context("worker stopped")?));
        }
        for _ in total_capacity..job_ids.len() {
            permits.add_permits(1);
            ensure!(executions.insert(starts.recv().await.context("worker stopped")?));
        }
        permits.add_permits(total_capacity);
        wait_for_successes(&database, job_ids.len() as u64).await?;
        ensure!(
            executions == job_ids,
            "jobs were lost or executed more than once"
        );
        ensure!(
            peak_active
                .iter()
                .all(|peak| peak.load(Ordering::SeqCst) == settings.concurrency)
        );
        Ok::<(), anyhow::Error>(())
    })
    .await;
    shutdown.cancel();
    permits.add_permits(job_ids.len());
    let mut stopped = Vec::new();
    for worker in workers {
        stopped.push(tokio::time::timeout(Duration::from_secs(5), worker).await);
    }
    for connection in connections {
        connection.close().await?;
    }
    let cleanup = database.finish().await;
    result.context("ready jobs waited for the empty-queue polling interval")??;
    for stopped in stopped {
        stopped???;
    }
    cleanup
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn empty_queue_waits_between_queries_and_discovers_later_jobs() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut connection = Database::connect(database.url.as_str()).await?;
    let (query_completed, mut query_completions) = mpsc::unbounded_channel();
    connection.set_metric_callback({
        move |metric| {
            if metric.statement.sql.contains("SKIP LOCKED") {
                query_completed
                    .send(std::time::Instant::now())
                    .expect("query receiver is open");
            }
        }
    });
    let shutdown = CancellationToken::new();
    let (started, mut starts) = mpsc::unbounded_channel();
    let worker = tokio::spawn(jobs::run(
        PostgresQueue::new(connection.clone()),
        jobs::WorkerSettings {
            poll_milliseconds: 1_000,
            ..Default::default()
        },
        shutdown.clone(),
        move |claim| {
            let started = started.clone();
            async move {
                started.send(claim.id).expect("start receiver is open");
                Ok(())
            }
        },
    ));
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let first_lookup = query_completions
            .recv()
            .await
            .context("first lookup missing")?;
        let job_id = queued_job(&database).await?;
        let next_lookup = query_completions
            .recv()
            .await
            .context("next lookup missing")?;
        ensure!(
            next_lookup.duration_since(first_lookup) >= Duration::from_secs(1),
            "empty queue was polled without waiting"
        );
        ensure!(
            starts.recv().await == Some(job_id),
            "later work was not discovered"
        );
        wait_for_successes(&database, 1).await
    })
    .await;
    shutdown.cancel();
    let stopped = tokio::time::timeout(Duration::from_secs(5), worker).await;
    connection.close().await?;
    let cleanup = database.finish().await;
    result??;
    stopped???;
    cleanup
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn expired_claim_recovery_does_not_hide_ready_jobs() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let expired_id = queued_job(&database).await?;
    let expired = queue
        .claim(30, 120)
        .await?
        .context("expired fixture missing")?;
    job::Entity::update_many()
        .col_expr(
            job::Column::LeaseUntil,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(expired_id))
        .exec(&database.connection)
        .await?;
    let ready_id = queued_job(&database).await?;
    let next = queue.claim(30, 120).await?;
    let result = next.as_ref().map(|claim| claim.id);
    if let Some(next) = next {
        queue.finish(&next, None).await?;
    }
    let recovered = job::Entity::find_by_id(expired_id)
        .one(&database.connection)
        .await?
        .context("recovered job missing")?;
    let stale_completion = queue.finish(&expired, None).await;
    database.finish().await?;
    ensure!(
        result == Some(ready_id),
        "lease recovery reported an empty queue before ready work"
    );
    ensure!(recovered.state == jobs::JobState::Pending);
    ensure!(matches!(stale_completion, Err(jobs::JobError::LeaseLost)));
    Ok(())
}

async fn wait_for_successes(database: &TestDatabase, count: u64) -> anyhow::Result<()> {
    loop {
        let succeeded = job::Entity::find()
            .filter(job::Column::State.eq(jobs::JobState::Succeeded))
            .count(&database.connection)
            .await?;
        if succeeded == count {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
