mod cleanup;
mod cleanup_orphaned_blobs;
pub mod sitemap;

use crate::DbPool;
use crate::config::WorkerConfig;
use crate::connection::R2Client;
use crate::connection::SeaweedFsClient;
use chrono_tz::Tz;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobBuilder, JobScheduler, JobSchedulerError};

/// Cleanup cron schedule: 4:00 AM every Saturday
/// Format: "sec min hour day month weekday"
const CLEANUP_SCHEDULE: &str = "0 0 4 * * 6";

/// Sitemap cron schedule: 3:00 AM every Sunday
const SITEMAP_SCHEDULE: &str = "0 0 3 * * 0";

/// Orphaned blob cleanup schedule: 5:00 AM every Friday
const ORPHANED_BLOB_CLEANUP_SCHEDULE: &str = "0 0 5 * * 5";

/// Create and start the cron scheduler
pub async fn start_scheduler(
    db_pool: DbPool,
    r2_client: R2Client,
    storage_client: SeaweedFsClient,
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

    // Register cleanup job
    tracing::info!(
        schedule = CLEANUP_SCHEDULE,
        timezone = %timezone,
        "Registering cleanup cron job"
    );
    let cleanup_job = create_cleanup_job(db_pool.clone(), timezone)?;
    sched.add(cleanup_job).await?;

    // Register sitemap job
    tracing::info!(
        schedule = SITEMAP_SCHEDULE,
        timezone = %timezone,
        "Registering sitemap cron job"
    );
    let sitemap_job = create_sitemap_job(db_pool.clone(), r2_client, config, timezone)?;
    sched.add(sitemap_job).await?;

    // Register orphaned blob cleanup job
    tracing::info!(
        schedule = ORPHANED_BLOB_CLEANUP_SCHEDULE,
        timezone = %timezone,
        "Registering orphaned blob cleanup cron job"
    );
    let orphan_cleanup_job = create_orphaned_blob_cleanup_job(db_pool, storage_client, timezone)?;
    sched.add(orphan_cleanup_job).await?;

    sched.start().await?;

    Ok(sched)
}

fn create_cleanup_job(db_pool: DbPool, timezone: Tz) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(CLEANUP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            Box::pin(async move {
                cleanup::run_cleanup(&db).await;
            })
        }))
        .build()
}

fn create_sitemap_job(
    db_pool: DbPool,
    r2_client: R2Client,
    config: &'static WorkerConfig,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(SITEMAP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let r2 = r2_client.clone();
            Box::pin(async move {
                sitemap::generate_and_upload_sitemap(&db, &r2, config).await;
            })
        }))
        .build()
}

fn create_orphaned_blob_cleanup_job(
    db_pool: DbPool,
    storage_client: SeaweedFsClient,
    timezone: Tz,
) -> Result<Job, JobSchedulerError> {
    let db = Arc::clone(&db_pool);

    JobBuilder::new()
        .with_timezone(timezone)
        .with_cron_job_type()
        .with_schedule(ORPHANED_BLOB_CLEANUP_SCHEDULE)?
        .with_run_async(Box::new(move |_uuid, _lock| {
            let db = Arc::clone(&db);
            let storage = storage_client.clone();
            Box::pin(async move {
                cleanup_orphaned_blobs::run_cleanup_orphaned_blobs(&db, &storage).await;
            })
        }))
        .build()
}
