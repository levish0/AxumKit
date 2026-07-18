use entity::board_posts::{Entity as PostEntity, Model as PostModel};
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};
use uuid::Uuid;

pub async fn repository_get_board_post_by_id<C>(conn: &C, id: Uuid) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    Ok(post)
}

/// Fetch a single board post by ID with a row lock (`SELECT ... FOR UPDATE`).
///
/// Used to serialize concurrent mutations of the same post (e.g. so its
/// `comment_count` is adjusted exactly once under concurrent deletes).
///
/// # Errors
/// - `Errors::BoardPostNotFound` if the post does not exist.
/// - DB/storage error on query failure.
pub async fn repository_get_board_post_by_id_for_update<C>(
    conn: &C,
    id: Uuid,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .lock_exclusive()
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    Ok(post)
}
