use axumkit_constants::POST_CONTENT_PREFIX;
use axumkit_dto::posts::CreatePostResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::bridge::worker_client::index_post;
use crate::connection::seaweedfs_conn::SeaweedFsClient;
use crate::repository::posts::repository_create_post;
use crate::state::WorkerClient;

pub async fn service_create_post(
    conn: &DatabaseConnection,
    seaweedfs_client: &SeaweedFsClient,
    worker: &WorkerClient,
    author_id: Uuid,
    title: String,
    content: String,
) -> Result<CreatePostResponse, Errors> {
    // 1. Generate storage key
    let post_id = Uuid::now_v7();
    let storage_key = format!("{}/{}", POST_CONTENT_PREFIX, post_id);

    // 2. Upload content to SeaweedFS
    seaweedfs_client
        .upload_content(&storage_key, &content)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload post content: {}", e);
            Errors::FileUploadError(e.to_string())
        })?;

    // 3. Create post in database with storage_key
    let post = repository_create_post(conn, author_id, title, storage_key).await?;

    // 4. Publish index job via bridge
    if let Err(e) = index_post(worker, post.id).await {
        tracing::warn!("Failed to queue post index job: {:?}", e);
    }

    Ok(CreatePostResponse {
        id: post.id.to_string(),
    })
}
