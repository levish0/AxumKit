use axumkit_dto::posts::PostResponse;
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::connection::seaweedfs_conn::SeaweedFsClient;
use crate::repository::posts::repository_get_post_by_id;

pub async fn service_get_post(
    conn: &DatabaseConnection,
    seaweedfs_client: &SeaweedFsClient,
    id: Uuid,
) -> Result<PostResponse, Errors> {
    let post = repository_get_post_by_id(conn, id).await?;

    // content를 SeaweedFS에서 읽어옴
    let content = seaweedfs_client
        .download_content(&post.storage_key)
        .await
        .map_err(|e| {
            tracing::error!("Failed to download post content: {}", e);
            Errors::FileReadError(e.to_string())
        })?;

    Ok(PostResponse {
        id: post.id.to_string(),
        author_id: post.author_id.to_string(),
        title: post.title,
        content,
        created_at: post.created_at,
        updated_at: post.updated_at,
    })
}
