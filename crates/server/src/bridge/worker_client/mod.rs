mod cache;
mod email;
mod index;
mod oauth;
mod reindex;

// Re-export all functions for backwards compatibility
pub use email::*;
pub use index::*;
pub use oauth::*;
pub use reindex::*;

use crate::state::WorkerClient;
use errors::errors::Errors;
use serde::Serialize;
use tracing::error;

/// Publish a job to NATS JetStream
pub async fn publish_job<T: Serialize>(
    worker: &WorkerClient,
    subject: &str,
    job: &T,
) -> Result<(), Errors> {
    let payload = serde_json::to_vec(job).map_err(|e| {
        error!(error = %e, subject, "Worker job serialization failed");
        Errors::WorkerServiceConnectionFailed
    })?;

    worker
        .publish(subject.to_string(), payload.into())
        .await
        .map_err(|e| {
            error!(error = %e, subject, "Worker job publish failed");
            Errors::WorkerServiceConnectionFailed
        })?
        .await
        .map_err(|e| {
            error!(error = %e, subject, "Worker job ack failed");
            Errors::WorkerServiceConnectionFailed
        })?;

    Ok(())
}
