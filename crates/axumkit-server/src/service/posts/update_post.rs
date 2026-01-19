use axumkit_dto::posts::PostResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::bridge::worker_client::index_post;
use crate::connection::seaweedfs_conn::SeaweedFsClient;
use crate::repository::posts::{
    PostUpdateParams, repository_get_post_by_id, repository_update_post,
};
use crate::state::WorkerClient;

pub async fn service_update_post(
    conn: &DatabaseConnection,
    seaweedfs_client: &SeaweedFsClient,
    worker: &WorkerClient,
    id: Uuid,
    author_id: Uuid,
    title: Option<String>,
    content: Option<String>,
) -> Result<PostResponse, Errors> {
    let post = repository_get_post_by_id(conn, id).await?;

    // 작성자만 수정 가능
    if post.author_id != author_id {
        return Err(Errors::UserUnauthorized);
    }

    // content가 변경되면 SeaweedFS에 업로드
    let new_storage_key = if let Some(ref new_content) = content {
        seaweedfs_client
            .upload_content(&post.storage_key, new_content)
            .await
            .map_err(|e| {
                tracing::error!("Failed to upload post content: {}", e);
                Errors::FileUploadError(e.to_string())
            })?;
        None // 기존 storage_key 유지
    } else {
        None
    };

    let params = PostUpdateParams {
        title,
        storage_key: new_storage_key,
    };
    let updated_post = repository_update_post(conn, post, params).await?;

    // Publish index job via bridge
    if let Err(e) = index_post(worker, updated_post.id).await {
        tracing::warn!("Failed to queue post index job: {:?}", e);
    }

    // content를 SeaweedFS에서 읽어옴
    let content = seaweedfs_client
        .download_content(&updated_post.storage_key)
        .await
        .map_err(|e| {
            tracing::error!("Failed to download post content: {}", e);
            Errors::FileReadError(e.to_string())
        })?;

    Ok(PostResponse {
        id: updated_post.id.to_string(),
        author_id: updated_post.author_id.to_string(),
        title: updated_post.title,
        content,
        created_at: updated_post.created_at,
        updated_at: updated_post.updated_at,
    })
}
