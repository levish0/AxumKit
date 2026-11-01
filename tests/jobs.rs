use jobs::{
    PostgresQueue, TraceContext,
    entities::{job, job_attempt},
};
use sea_orm::{
    ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait, sea_query::Expr,
};
use std::time::Duration;
use test_support::TestDatabase;
use uuid::Uuid;

#[path = "jobs/retention.rs"]
mod retention;

#[path = "jobs/workers.rs"]
mod workers;

#[path = "jobs/dispatch.rs"]
mod dispatch;

#[path = "jobs/scheduling.rs"]
mod scheduling;

#[path = "jobs/provider_retry.rs"]
mod provider_retry;

#[path = "jobs/pressure.rs"]
mod pressure;

async fn queued_job(database: &TestDatabase) -> anyhow::Result<Uuid> {
    let transaction = database.connection.begin().await?;
    let job_id = Uuid::now_v7();
    jobs::enqueue(
        &transaction,
        job_id,
        "test.execution",
        &serde_json::json!({}),
        TraceContext::default(),
    )
    .await?;
    transaction.commit().await?;
    Ok(job_id)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn shutdown_stops_claiming_and_bounds_drain_of_a_hung_handler() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let job_id = queued_job(&database).await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let shutdown = tokio_util::sync::CancellationToken::new();
    let started = std::sync::Arc::new(tokio::sync::Notify::new());
    let handler_started = started.clone();
    let settings = jobs::WorkerSettings {
        concurrency: 1,
        lease_seconds: 3,
        execution_seconds: 10,
        drain_seconds: 1,
        poll_milliseconds: 25,
    };
    let worker = tokio::spawn(jobs::run(queue, settings, shutdown.clone(), move |_| {
        let started = handler_started.clone();
        async move {
            started.notify_one();
            std::future::pending::<Result<(), jobs::HandlerFailure>>().await
        }
    }));
    tokio::time::timeout(Duration::from_secs(10), started.notified()).await?;
    shutdown.cancel();
    tokio::time::timeout(Duration::from_secs(3), worker).await???;
    assert_eq!(
        job::Entity::find_by_id(job_id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        jobs::JobState::Running
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn retry_delay_and_success_retention_preserve_failed_jobs() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let job_id = queued_job(&database).await?;
    let first = queue.claim(10, 30).await?.unwrap();
    queue
        .finish(&first, Some(("temporary_failure", true)))
        .await?;
    assert!(queue.claim(10, 30).await?.is_none());
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let retry = queue.claim(10, 30).await?.unwrap();
    assert_eq!(retry.attempts, 2);
    queue.finish(&retry, None).await?;
    let failed_id = queued_job(&database).await?;
    let permanent = queue.claim(10, 30).await?.unwrap();
    queue
        .finish(&permanent, Some(("permanent_failure", false)))
        .await?;
    job::Entity::update_many()
        .col_expr(
            job::Column::FinishedAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::days(31)),
        )
        .filter(job::Column::Id.is_in([job_id, failed_id]))
        .exec(&database.connection)
        .await?;
    assert_eq!(queue.cleanup_finished(30, 90).await?, 1);
    assert!(
        job::Entity::find_by_id(job_id)
            .one(&database.connection)
            .await?
            .is_none()
    );
    assert_eq!(
        job_attempt::Entity::find()
            .filter(job_attempt::Column::JobId.eq(job_id))
            .count(&database.connection)
            .await?,
        0
    );
    assert_eq!(
        job::Entity::find_by_id(failed_id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        jobs::JobState::Failed
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn concurrent_claims_fence_expired_attempts_and_preserve_history() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    queued_job(&database).await?;
    let started = std::time::Instant::now();
    let (left, right) = tokio::join!(queue.claim(10, 30), queue.claim(10, 30));
    let (left, right) = (left?, right?);
    assert_eq!(
        usize::from(left.is_some()) + usize::from(right.is_some()),
        1,
        "claim duration: {:?}",
        started.elapsed()
    );
    let expired = left.or(right).unwrap();
    job::Entity::update_many()
        .col_expr(
            job::Column::LeaseUntil,
            Expr::value(chrono::Utc::now() - chrono::Duration::seconds(1)),
        )
        .filter(job::Column::Id.eq(expired.id))
        .exec(&database.connection)
        .await?;
    assert!(queue.claim(1, 5).await?.is_none());
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let replacement = queue.claim(1, 5).await?.unwrap();
    assert_ne!(expired.claim_token, replacement.claim_token);
    assert!(matches!(
        queue.finish(&expired, None).await,
        Err(jobs::JobError::LeaseLost)
    ));
    queue.finish(&replacement, None).await?;
    assert_eq!(
        job_attempt::Entity::find()
            .count(&database.connection)
            .await?,
        2
    );
    assert_eq!(
        job_attempt::Entity::find_by_id(expired.claim_token)
            .one(&database.connection)
            .await?
            .unwrap()
            .outcome,
        Some(jobs::AttemptOutcome::LeaseExpired)
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn final_attempt_crash_becomes_retained_failure_and_can_be_retried() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    let job_id = queued_job(&database).await?;
    job::Entity::update_many()
        .col_expr(job::Column::MaxAttempts, Expr::value(1))
        .filter(job::Column::Id.eq(job_id))
        .exec(&database.connection)
        .await?;
    queue.claim(1, 5).await?.unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(queue.claim(1, 5).await?.is_none());
    assert_eq!(
        job::Entity::find_by_id(job_id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        jobs::JobState::Failed
    );
    assert!(queue.retry_failed(job_id).await?);
    let retried = queue.claim(1, 5).await?.unwrap();
    assert_eq!(retried.attempts, 2);
    queue.finish(&retried, None).await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn heartbeat_keeps_a_slow_successful_handler_exclusive() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    queued_job(&database).await?;
    let claim = queue.claim(9, 30).await?.unwrap();
    let settings = jobs::WorkerSettings {
        lease_seconds: 9,
        execution_seconds: 30,
        ..Default::default()
    };
    let release_handler = std::sync::Arc::new(tokio::sync::Notify::new());
    let handler_release = release_handler.clone();
    let worker_queue = queue.clone();
    let execution = tokio::spawn(async move {
        jobs::run_one(&worker_queue, claim, settings, async {
            handler_release.notified().await;
            Ok(())
        })
        .await
    });
    tokio::time::sleep(Duration::from_millis(9500)).await;
    assert!(queue.claim(9, 30).await?.is_none());
    release_handler.notify_one();
    execution.await??;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-db"]
async fn heartbeat_cannot_hold_a_hung_handler_forever() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let queue = PostgresQueue::new(database.connection.clone());
    queued_job(&database).await?;
    let claim = queue.claim(3, 3).await?.unwrap();
    let settings = jobs::WorkerSettings {
        lease_seconds: 3,
        execution_seconds: 3,
        ..Default::default()
    };
    let _ = jobs::run_one(&queue, claim, settings, std::future::pending()).await;
    assert!(queue.claim(3, 3).await?.is_none());
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let replacement = queue
        .claim(3, 3)
        .await?
        .expect("deadline must permit recovery after backoff");
    assert_eq!(replacement.attempts, 2);
    queue.finish(&replacement, None).await?;
    database.finish().await
}
