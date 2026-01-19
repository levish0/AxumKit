use axumkit_dto::posts::DeletePostResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::bridge::worker_client::{delete_content, delete_post_from_index};
use crate::repository::posts::{repository_delete_post, repository_get_post_by_id};
use crate::state::WorkerClient;

pub async fn service_delete_post(
    conn: &DatabaseConnection,
    worker: &WorkerClient,
    id: Uuid,
    author_id: Uuid,
) -> Result<DeletePostResponse, Errors> {
    let post = repository_get_post_by_id(conn, id).await?;

    // 작성자만 삭제 가능
    if post.author_id != author_id {
        return Err(Errors::UserUnauthorized);
    }

    let storage_key = post.storage_key.clone();

    // DB에서 삭제
    repository_delete_post(conn, id).await?;

    // Index에서 삭제
    if let Err(e) = delete_post_from_index(worker, id).await {
        tracing::warn!("Failed to queue post delete from index: {:?}", e);
    }

    // Storage에서 삭제 (worker queue)
    if let Err(e) = delete_content(worker, vec![storage_key]).await {
        tracing::warn!("Failed to queue content delete: {:?}", e);
    }

    Ok(DeletePostResponse { success: true })
}
