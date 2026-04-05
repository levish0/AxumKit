use super::publish_job;
use crate::state::WorkerClient;
use axumkit_errors::errors::Errors;
use axumkit_worker::jobs::reindex::create_reindex_users_job;
use axumkit_worker::nats::streams::REINDEX_USERS_SUBJECT;
use tracing::info;
use uuid::Uuid;

/// Start a full reindex of all users
pub async fn start_reindex_users(
    worker: &WorkerClient,
    batch_size: Option<u32>,
) -> Result<Uuid, Errors> {
    let reindex_id = Uuid::now_v7();
    info!(
        reindex_id = %reindex_id,
        batch_size = ?batch_size,
        "Starting user reindex job"
    );

    let job = create_reindex_users_job(reindex_id, batch_size);

    publish_job(worker, REINDEX_USERS_SUBJECT, &job).await?;

    info!(reindex_id = %reindex_id, "User reindex job started");
    Ok(reindex_id)
}
