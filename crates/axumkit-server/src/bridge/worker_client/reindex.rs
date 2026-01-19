use super::publish_job;
use crate::state::WorkerClient;
use axumkit_errors::errors::Errors;
use axumkit_worker::jobs::reindex::{create_reindex_posts_job, create_reindex_users_job};
use axumkit_worker::nats::streams::{REINDEX_POSTS_SUBJECT, REINDEX_USERS_SUBJECT};
use tracing::info;
use uuid::Uuid;

/// Start a full reindex of all posts
pub async fn start_reindex_posts(
    worker: &WorkerClient,
    batch_size: Option<u32>,
) -> Result<Uuid, Errors> {
    let reindex_id = Uuid::now_v7();
    info!(
        "Starting post reindex job: reindex_id={}, batch_size={:?}",
        reindex_id, batch_size
    );

    let job = create_reindex_posts_job(reindex_id, batch_size);

    publish_job(worker, REINDEX_POSTS_SUBJECT, &job).await?;

    info!("Post reindex job started: reindex_id={}", reindex_id);
    Ok(reindex_id)
}

/// Start a full reindex of all users
pub async fn start_reindex_users(
    worker: &WorkerClient,
    batch_size: Option<u32>,
) -> Result<Uuid, Errors> {
    let reindex_id = Uuid::now_v7();
    info!(
        "Starting user reindex job: reindex_id={}, batch_size={:?}",
        reindex_id, batch_size
    );

    let job = create_reindex_users_job(reindex_id, batch_size);

    publish_job(worker, REINDEX_USERS_SUBJECT, &job).await?;

    info!("User reindex job started: reindex_id={}", reindex_id);
    Ok(reindex_id)
}
