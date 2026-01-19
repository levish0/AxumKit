use axumkit_entity::posts::Model as PostModel;
use axumkit_errors::errors::Errors;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use super::find_by_id::repository_find_post_by_id;

pub async fn repository_get_post_by_id<C>(conn: &C, id: Uuid) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    repository_find_post_by_id(conn, id)
        .await?
        .ok_or(Errors::PostNotFound)
}
