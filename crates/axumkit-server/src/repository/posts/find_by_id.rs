use axumkit_entity::posts::{Entity as PostEntity, Model as PostModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

pub async fn repository_find_post_by_id<C>(conn: &C, id: Uuid) -> Result<Option<PostModel>, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id).one(conn).await?;
    Ok(post)
}
