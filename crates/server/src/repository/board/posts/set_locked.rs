use entity::board_posts::{
    ActiveModel as PostActiveModel, Entity as PostEntity, Model as PostModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

/// Set a board post's `is_locked` flag. Moderation-only (gated in the service layer).
pub async fn repository_set_board_post_locked<C>(
    conn: &C,
    id: Uuid,
    locked: bool,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    let mut active: PostActiveModel = post.into();
    active.is_locked = Set(locked);
    Ok(active.update(conn).await?)
}
