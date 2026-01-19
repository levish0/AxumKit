use axumkit_entity::posts::{Column, Entity as PostEntity, Model as PostModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, QueryOrder, QuerySelect};

pub async fn repository_list_posts<C>(
    conn: &C,
    limit: u64,
    offset: u64,
) -> Result<Vec<PostModel>, Errors>
where
    C: ConnectionTrait,
{
    let posts = PostEntity::find()
        .order_by_desc(Column::CreatedAt)
        .offset(offset)
        .limit(limit)
        .all(conn)
        .await?;

    Ok(posts)
}
