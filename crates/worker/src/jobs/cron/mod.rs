mod cleanup;
mod cleanup_expired_roles;
mod cleanup_old_notifications;
mod expiry;
mod flush_board_view_counts;
pub mod sitemap;

use crate::CacheClient;
use crate::DbPool;
use crate::LockClient;
use chrono_tz::Tz;
use config::WorkerConfig;
use redis::Script;
use std::pin::pin;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use storage::R2AssetsClient;
use tokio_cron_scheduler::{Job, JobBuilder, JobScheduler, JobSchedulerError};
use uuid::Uuid;

/// Cleanup cron schedule: 4:00 AM every Saturday
/// Format: "sec min hour day month weekday"
const CLEANUP_SCHEDULE: &str = "0 0 4 * * 6";

/// Sitemap cron schedule: 3:00 AM every Sunday
const SITEMAP_SCHEDULE: &str = "0 0 3 * * 0";

/// Board view-count flush schedule: every minute (at second 0).
///
/// Frequent enough that displayed counts lag by at most ~a minute; each run is
/// a single HGETALL+DEL drain plus one UPDATE per touched post, so an idle
/// minute costs one empty Redis roundtrip.
const FLUSH_BOARD_VIEW_COUNTS_SCHEDULE: &str = "0 * * * * *";

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
    lock_client: LockClient,
    cache_client: CacheClient,
    r2_assets: R2AssetsClient,
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
    let sitemap_job =
        create_sitemap_job(db_pool.clone(), lock_client, r2_assets, config, timezone)?;
    sched.add(sitemap_job).await?;

    tracing::info!(
        schedule = FLUSH_BOARD_VIEW_COUNTS_SCHEDULE,
        timezone = %timezone,
        "Registering board view-count flush cron job"
    );
    let flush_views_job = create_flush_board_view_counts_job(db_pool, cache_client, timezone)?;
    sched.add(flush_views_job).await?;

    sched.start().await?;

    Ok(sched)
}

fn create_cleanup_job(
    db_pool: DbPool,
    lock_client: LockClient,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);
    let lock = lock_client.clone();

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(CLEANUP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let lock = lock.clone();
            Box::pin(async move {
                run_with_cron_lock(lock, CLEANUP_LOCK_KEY, "cleanup", || async move {
                    cleanup::run_cleanup(&db).await;
                })
                .await;
            })
        }))
        .build()
}

fn create_sitemap_job(
    db_pool: DbPool,
    lock_client: LockClient,
    r2_assets: R2AssetsClient,
    config: &'static WorkerConfig,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);
    let lock = lock_client.clone();

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(SITEMAP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let lock = lock.clone();
            let r2 = r2_assets.clone();
            Box::pin(async move {
                run_with_cron_lock(lock, SITEMAP_LOCK_KEY, "sitemap", || async move {
                    sitemap::generate_and_upload_sitemap(&db, &r2, config).await;
                })
                .await;
            })
        }))
        .build()
}

async fn run_with_cron_lock<F, Fut>(
    lock_client: LockClient,
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
    lock_client: &LockClient,
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
    lock_client: &LockClient,
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
    lock_client: &LockClient,
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

fn create_flush_board_view_counts_job(
    db_pool: DbPool,
    cache_client: CacheClient,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);
    let cache = cache_client.clone();

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(FLUSH_BOARD_VIEW_COUNTS_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let cache = cache.clone();
            Box::pin(async move {
                // No cron lock: the atomic drain (HGETALL + DEL) ensures only one
                // worker applies a given batch even with multiple instances.
                flush_board_view_counts::run_flush_board_view_counts(&db, &cache).await;
            })
        }))
        .build()
}
