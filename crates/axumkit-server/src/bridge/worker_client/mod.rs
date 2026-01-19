mod cache;
mod email;
mod index;
mod reindex;
mod storage;

// Re-export all functions for backwards compatibility
pub use email::*;
pub use index::*;
pub use reindex::*;
pub use storage::*;

use crate::state::WorkerClient;
use axumkit_errors::errors::Errors;
use serde::Serialize;

/// Publish a job to NATS JetStream
pub async fn publish_job<T: Serialize>(
    worker: &WorkerClient,
    subject: &str,
    job: &T,
) -> Result<(), Errors> {
    let payload = serde_json::to_vec(job).map_err(|_| Errors::WorkerServiceConnectionFailed)?;

    worker
        .publish(subject.to_string(), payload.into())
        .await
        .map_err(|_| Errors::WorkerServiceConnectionFailed)?;

    Ok(())
}
