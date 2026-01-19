use super::publish_job;
use crate::state::WorkerClient;
use axumkit_errors::errors::Errors;
use axumkit_worker::jobs::{
    post_index::{IndexAction, IndexPostJob},
    user_index::{IndexUserJob, UserIndexAction},
};
use axumkit_worker::nats::streams::{INDEX_POST_SUBJECT, INDEX_USER_SUBJECT};
use tracing::info;
use uuid::Uuid;

/// Push a post indexing job to the worker queue
pub async fn index_post(worker: &WorkerClient, post_id: Uuid) -> Result<(), Errors> {
    info!("Queuing post index job for {}", post_id);

    let job = IndexPostJob {
        post_id,
        action: IndexAction::Index,
    };

    publish_job(worker, INDEX_POST_SUBJECT, &job).await?;

    info!("Post index job queued for {}", post_id);
    Ok(())
}

/// Push a post deletion job to the worker queue
pub async fn delete_post_from_index(worker: &WorkerClient, post_id: Uuid) -> Result<(), Errors> {
    info!("Queuing post delete job for {}", post_id);

    let job = IndexPostJob {
        post_id,
        action: IndexAction::Delete,
    };

    publish_job(worker, INDEX_POST_SUBJECT, &job).await?;

    info!("Post delete job queued for {}", post_id);
    Ok(())
}

/// Push a user indexing job to the worker queue
pub async fn index_user(worker: &WorkerClient, user_id: Uuid) -> Result<(), Errors> {
    info!("Queuing user index job for {}", user_id);

    let job = IndexUserJob {
        user_id,
        action: UserIndexAction::Index,
    };

    publish_job(worker, INDEX_USER_SUBJECT, &job).await?;

    info!("User index job queued for {}", user_id);
    Ok(())
}

/// Push a user deletion job to the worker queue
pub async fn delete_user_from_index(worker: &WorkerClient, user_id: Uuid) -> Result<(), Errors> {
    info!("Queuing user delete job for {}", user_id);

    let job = IndexUserJob {
        user_id,
        action: UserIndexAction::Delete,
    };

    publish_job(worker, INDEX_USER_SUBJECT, &job).await?;

    info!("User delete job queued for {}", user_id);
    Ok(())
}
