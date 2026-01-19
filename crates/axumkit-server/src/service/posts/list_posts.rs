use axumkit_dto::posts::{ListPostsResponse, PostListItem};
use axumkit_errors::errors::Errors;
use sea_orm::DatabaseConnection;

use crate::repository::posts::repository_list_posts;

pub async fn service_list_posts(
    conn: &DatabaseConnection,
    limit: u64,
    offset: u64,
) -> Result<ListPostsResponse, Errors> {
    let posts = repository_list_posts(conn, limit, offset).await?;
    let posts = posts
        .into_iter()
        .map(|post| PostListItem {
            id: post.id.to_string(),
            author_id: post.author_id.to_string(),
            title: post.title,
            created_at: post.created_at,
            updated_at: post.updated_at,
        })
        .collect();
    Ok(ListPostsResponse { posts })
}
