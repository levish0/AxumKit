use crate::{bootstrap::Application, config::RetentionSettings};
use application::ApplicationResult;
use jobs::{Claim, HandlerFailure, JobError, PostgresQueue, TraceContext};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const MAINTENANCE_QUEUE: &str = "maintenance";
pub const MAINTENANCE_JOB: &str = "system.maintenance";

#[tracing::instrument(skip_all)]
pub async fn cleanup(app: &Application, retention: &RetentionSettings) -> ApplicationResult<()> {
    let authentication_records = app.identity.cleanup_expired_authentication().await?;
    let completed_jobs = app
        .queue
        .cleanup_finished(retention.succeeded_job_days, retention.failed_job_days)
        .await?;
    let audit_events = app
        .identity
        .cleanup_audit_events(retention.audit_days)
        .await?;
    let file_records = app.files.reconcile_files().await?;
    let notification_records = app
        .notifications
        .cleanup(retention.notification_days)
        .await?;
    let api_usage_records = app
        .api_access
        .cleanup_usage(retention.api_usage_days)
        .await?;
    tracing::info!(
        authentication_records,
        completed_jobs,
        audit_events,
        file_records,
        notification_records,
        api_usage_records,
        "expired records removed"
    );
    Ok(())
}

pub async fn execute_job(
    app: &Application,
    retention: &RetentionSettings,
    claim: &Claim,
) -> Result<(), HandlerFailure> {
    if claim.kind != MAINTENANCE_JOB {
        return Err(HandlerFailure {
            code: "unknown_maintenance_job",
            retryable: false,
        });
    }
    cleanup(app, retention).await.map_err(|_| HandlerFailure {
        code: "maintenance_cleanup_failed",
        retryable: true,
    })?;
    crate::integrations::repair_search(app)
        .await
        .map_err(|_| HandlerFailure {
            code: "search_repair_failed",
            retryable: true,
        })?;
    Ok(())
}

pub async fn schedule(
    queue: PostgresQueue,
    interval_seconds: i64,
    shutdown: CancellationToken,
) -> Result<(), JobError> {
    if !(1..=86400).contains(&interval_seconds) {
        return Err(JobError::Invalid);
    }
    let mut poll = tokio::time::interval(Duration::from_secs(interval_seconds.min(15) as u64));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => return Ok(()),
            _ = poll.tick() => {
                let inserted = tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => return Ok(()),
                    result = queue.schedule_periodic(MAINTENANCE_JOB, interval_seconds, TraceContext::default()) => result?,
                };
                if inserted { tracing::info!(job_kind = MAINTENANCE_JOB, "periodic maintenance registered"); }
            }
        }
    }
}
