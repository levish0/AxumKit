use crate::{Claim, JobError, PostgresQueue};
use std::{future::Future, time::Duration};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::Instrument;

#[derive(Clone, Copy)]
pub struct WorkerSettings {
    pub concurrency: usize,
    pub lease_seconds: i32,
    pub execution_seconds: i32,
    pub drain_seconds: u64,
    /// Delay between lookups after no eligible work is found.
    pub poll_milliseconds: u64,
}

impl Default for WorkerSettings {
    fn default() -> Self {
        Self {
            concurrency: 4,
            lease_seconds: 30,
            execution_seconds: 120,
            drain_seconds: 35,
            poll_milliseconds: 250,
        }
    }
}

impl WorkerSettings {
    pub fn validate(&self) -> Result<(), JobError> {
        if self.concurrency == 0
            || self.concurrency > 64
            || self.lease_seconds < 3
            || self.execution_seconds < self.lease_seconds
            || self.execution_seconds > 3600
            || self.poll_milliseconds == 0
            || self.drain_seconds == 0
        {
            return Err(JobError::Invalid);
        }
        Ok(())
    }
}

pub struct HandlerFailure {
    pub code: &'static str,
    pub retryable: bool,
}

pub async fn run_one<F: Future<Output = Result<(), HandlerFailure>>>(
    queue: &PostgresQueue,
    claim: Claim,
    settings: WorkerSettings,
    handler: F,
) -> Result<(), JobError> {
    let span = tracing::info_span!("job", job_id=%claim.id, attempt_id=%claim.claim_token,
        attempt=claim.attempts, job_kind=%claim.kind, request_id=%claim.request_id);
    async {
        let mut heartbeat =
            tokio::time::interval(Duration::from_secs((settings.lease_seconds / 3) as u64));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // The monotonic execution deadline also limits a heartbeat that waits.
        let execution = async {
            tokio::pin!(handler);
            loop {
                tokio::select! {
                    result = &mut handler => break Ok::<_, JobError>(result),
                    _ = heartbeat.tick() => queue.heartbeat(&claim, settings.lease_seconds).await?,
                }
            }
        };
        let result = tokio::time::timeout_at(claim.execution_deadline, execution).await;
        let failure = match result {
            Ok(Ok(Ok(()))) => None,
            Ok(Ok(Err(failure))) => Some((failure.code, failure.retryable)),
            Ok(Err(error)) => return Err(error),
            Err(_) => Some(("execution_timeout", true)),
        };
        tracing::info!(
            outcome = failure.map_or("succeeded", |(code, _)| code),
            "job execution finished"
        );
        queue.finish(&claim, failure).await
    }
    .instrument(span)
    .await
}

pub async fn run<H, F>(
    queue: PostgresQueue,
    settings: WorkerSettings,
    shutdown: CancellationToken,
    handler: H,
) -> Result<(), JobError>
where
    H: Fn(Claim) -> F + Clone + Send + Sync + 'static,
    F: Future<Output = Result<(), HandlerFailure>> + Send + 'static,
{
    settings.validate()?;
    let mut tasks = JoinSet::new();
    let empty_queue_poll_delay = Duration::from_millis(settings.poll_milliseconds);
    let mut next_claim_at = tokio::time::Instant::now();
    loop {
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => break,
            result = tasks.join_next(), if !tasks.is_empty() => {
                match result {
                    Some(Ok(Ok(()))) => {},
                    Some(Ok(Err(_))) => tracing::error!(error_code="job_storage_or_lease", "job finalization failed; lease recovery remains authoritative"),
                    Some(Err(_)) => tracing::error!(error_code="handler_panic", "job task failed; lease recovery remains authoritative"),
                    None => {},
                }
            },
            _ = tokio::time::sleep_until(next_claim_at), if tasks.len() < settings.concurrency => {
                let claim = tokio::select! {
                    biased;
                    _ = shutdown.cancelled() => break,
                    claim = queue.claim(settings.lease_seconds, settings.execution_seconds) => claim?,
                };
                if let Some(claim) = claim {
                    let queue = queue.clone();
                    let execution = handler(claim.clone());
                    tasks.spawn(async move { run_one(&queue, claim, settings, execution).await });
                    next_claim_at = tokio::time::Instant::now();
                } else {
                    next_claim_at = tokio::time::Instant::now() + empty_queue_poll_delay;
                }
            },
        }
    }
    let drain = async { while tasks.join_next().await.is_some() {} };
    if tokio::time::timeout(Duration::from_secs(settings.drain_seconds), drain)
        .await
        .is_err()
    {
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }
    Ok(())
}
