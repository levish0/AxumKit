use crate::bridge::worker_client;
use crate::state::WorkerClient;
use tracing::warn;
use uuid::Uuid;

pub fn spawn_index_user(worker: &WorkerClient, user_id: Uuid) {
    let worker = worker.clone();
    tokio::spawn(async move {
        if let Err(e) = worker_client::index_user(&worker, user_id).await {
            warn!(user_id = %user_id, error = ?e, "Failed to queue user index job");
        }
    });
}

pub fn spawn_delete_user_from_index(worker: &WorkerClient, user_id: Uuid) {
    let worker = worker.clone();
    tokio::spawn(async move {
        if let Err(e) = worker_client::delete_user_from_index(&worker, user_id).await {
            warn!(user_id = %user_id, error = ?e, "Failed to queue user index delete job");
        }
    });
}

pub fn spawn_oauth_profile_image(worker: &WorkerClient, user_id: Uuid, image_url: String) {
    let worker = worker.clone();
    tokio::spawn(async move {
        if let Err(e) =
            worker_client::process_oauth_profile_image(&worker, user_id, image_url).await
        {
            warn!(user_id = %user_id, error = ?e, "Failed to queue OAuth profile image job");
        }
    });
}
