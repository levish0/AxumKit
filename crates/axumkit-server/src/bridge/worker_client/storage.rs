use super::publish_job;
use crate::state::WorkerClient;
use axumkit_errors::errors::Errors;
use axumkit_worker::jobs::storage::DeleteContentJob;
use axumkit_worker::nats::streams::DELETE_CONTENT_SUBJECT;
use tracing::info;

/// Push a content deletion job to the worker queue
pub async fn delete_content(
    worker: &WorkerClient,
    storage_keys: Vec<String>,
) -> Result<(), Errors> {
    if storage_keys.is_empty() {
        return Ok(());
    }

    info!("Queuing content delete job for {} keys", storage_keys.len());

    let job = DeleteContentJob { storage_keys };

    publish_job(worker, DELETE_CONTENT_SUBJECT, &job).await?;

    info!("Content delete job queued");
    Ok(())
}
