use axumkit_entity::posts::{Column, Entity as PostEntity, Model as PostModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

pub async fn repository_find_posts_by_author<C>(
    conn: &C,
    author_id: Uuid,
) -> Result<Vec<PostModel>, Errors>
where
    C: ConnectionTrait,
{
    let posts = PostEntity::find()
        .filter(Column::AuthorId.eq(author_id))
        .order_by_desc(Column::CreatedAt)
        .all(conn)
        .await?;

    Ok(posts)
}
