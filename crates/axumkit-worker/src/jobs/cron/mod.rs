mod cleanup;
pub mod sitemap;

use crate::DbPool;
use crate::SessionClient;
use crate::config::WorkerConfig;
use crate::connection::R2Client;
use chrono_tz::Tz;
use redis::Script;
use std::pin::pin;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobBuilder, JobScheduler, JobSchedulerError};
use uuid::Uuid;

/// Cleanup cron schedule: 4:00 AM every Saturday
/// Format: "sec min hour day month weekday"
const CLEANUP_SCHEDULE: &str = "0 0 4 * * 6";

/// Sitemap cron schedule: 3:00 AM every Sunday
const SITEMAP_SCHEDULE: &str = "0 0 3 * * 0";

/// Distributed lock TTL for cron jobs (seconds).
const CRON_LOCK_TTL_SECONDS: u64 = 60 * 30; // 30 minutes
/// Heartbeat interval for lock extension (seconds).
const CRON_LOCK_HEARTBEAT_SECONDS: u64 = 60 * 10; // 10 minutes

const CLEANUP_LOCK_KEY: &str = "cron:lock:cleanup";
const SITEMAP_LOCK_KEY: &str = "cron:lock:sitemap";

static RELEASE_LOCK_SCRIPT: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("lua/release_lock.lua")));
static EXTEND_LOCK_SCRIPT: LazyLock<Script> =
    LazyLock::new(|| Script::new(include_str!("lua/extend_lock.lua")));

/// Create and start the cron scheduler.
pub async fn start_scheduler(
    db_pool: DbPool,
    lock_client: SessionClient,
    r2_client: R2Client,
    config: &'static WorkerConfig,
) -> Result<JobScheduler, JobSchedulerError> {
    let sched = JobScheduler::new().await?;

    let timezone: Tz = config.cron_timezone.parse().unwrap_or_else(|_| {
        tracing::warn!(
            timezone = %config.cron_timezone,
            "Invalid timezone, falling back to UTC"
        );
        chrono_tz::UTC
    });

    tracing::info!(
        schedule = CLEANUP_SCHEDULE,
        timezone = %timezone,
        "Registering cleanup cron job"
    );
    let cleanup_job = create_cleanup_job(db_pool.clone(), lock_client.clone(), timezone)?;
    sched.add(cleanup_job).await?;

    tracing::info!(
        schedule = SITEMAP_SCHEDULE,
        timezone = %timezone,
        "Registering sitemap cron job"
    );
    let sitemap_job = create_sitemap_job(db_pool, lock_client, r2_client, config, timezone)?;
    sched.add(sitemap_job).await?;

    sched.start().await?;

    Ok(sched)
}

fn create_cleanup_job(
    db_pool: DbPool,
    lock_client: SessionClient,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);
    let lock_client = lock_client.clone();

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(CLEANUP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let lock_client = lock_client.clone();
            Box::pin(async move {
                run_with_cron_lock(lock_client, CLEANUP_LOCK_KEY, "cleanup", || async move {
                    cleanup::run_cleanup(&db).await;
                })
                .await;
            })
        }))
        .build()
}

fn create_sitemap_job(
    db_pool: DbPool,
    lock_client: SessionClient,
    r2_client: R2Client,
    config: &'static WorkerConfig,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);
    let lock_client = lock_client.clone();

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(SITEMAP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let lock_client = lock_client.clone();
            let r2 = r2_client.clone();
            Box::pin(async move {
                run_with_cron_lock(lock_client, SITEMAP_LOCK_KEY, "sitemap", || async move {
                    sitemap::generate_and_upload_sitemap(&db, &r2, config).await;
                })
                .await;
            })
        }))
        .build()
}

async fn run_with_cron_lock<F, Fut>(
    lock_client: SessionClient,
    lock_key: &'static str,
    job_name: &'static str,
    run: F,
) where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let lock_token = Uuid::now_v7().to_string();
    match try_acquire_cron_lock(&lock_client, lock_key, &lock_token).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::info!(
                job = job_name,
                lock_key,
                "Skipping cron run; lock already held"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                job = job_name,
                lock_key,
                error = %e,
                "Cron lock unavailable; skipping run to avoid concurrent execution"
            );
            return;
        }
    }

    tracing::info!(job = job_name, lock_key, "Cron lock acquired; starting job");
    let mut job_future = pin!(run());
    let heartbeat_interval = Duration::from_secs(CRON_LOCK_HEARTBEAT_SECONDS);
    let mut lock_lost = false;

    loop {
        tokio::select! {
            _ = &mut job_future => break,
            _ = tokio::time::sleep(heartbeat_interval), if !lock_lost => {
                match extend_cron_lock(&lock_client, lock_key, &lock_token).await {
                    Ok(true) => {
                        tracing::debug!(
                            job = job_name,
                            lock_key,
                            ttl_seconds = CRON_LOCK_TTL_SECONDS,
                            "Cron lock heartbeat extended"
                        );
                    }
                    Ok(false) => {
                        lock_lost = true;
                        tracing::error!(
                            job = job_name,
                            lock_key,
                            "Cron lock heartbeat lost ownership; duplicate run may occur"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            job = job_name,
                            lock_key,
                            error = %e,
                            "Failed to extend cron lock heartbeat"
                        );
                    }
                }
            }
        }
    }

    if lock_lost {
        tracing::warn!(
            job = job_name,
            lock_key,
            "Cron job finished after lock ownership was lost"
        );
    }

    if let Err(e) = release_cron_lock(&lock_client, lock_key, &lock_token).await {
        tracing::warn!(job = job_name, lock_key, error = %e, "Failed to release cron lock");
    } else {
        tracing::info!(job = job_name, lock_key, "Cron lock released");
    }
}

async fn try_acquire_cron_lock(
    lock_client: &SessionClient,
    lock_key: &str,
    token: &str,
) -> Result<bool, redis::RedisError> {
    let mut conn = lock_client.as_ref().clone();
    let result: Option<String> = redis::cmd("SET")
        .arg(lock_key)
        .arg(token)
        .arg("NX")
        .arg("EX")
        .arg(CRON_LOCK_TTL_SECONDS)
        .query_async(&mut conn)
        .await?;

    Ok(matches!(result, Some(value) if value == "OK"))
}

async fn release_cron_lock(
    lock_client: &SessionClient,
    lock_key: &str,
    token: &str,
) -> Result<(), redis::RedisError> {
    let mut conn = lock_client.as_ref().clone();
    let _: i32 = RELEASE_LOCK_SCRIPT
        .key(lock_key)
        .arg(token)
        .invoke_async(&mut conn)
        .await?;
    Ok(())
}

async fn extend_cron_lock(
    lock_client: &SessionClient,
    lock_key: &str,
    token: &str,
) -> Result<bool, redis::RedisError> {
    let mut conn = lock_client.as_ref().clone();
    let result: i32 = EXTEND_LOCK_SCRIPT
        .key(lock_key)
        .arg(token)
        .arg(CRON_LOCK_TTL_SECONDS)
        .invoke_async(&mut conn)
        .await?;

    Ok(result == 1)
}
