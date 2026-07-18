use entity::board_posts::Entity as PostEntity;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, ModelTrait};
use uuid::Uuid;

pub async fn repository_delete_board_post<C>(conn: &C, id: Uuid) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    post.delete(conn).await?;

    Ok(())
}
