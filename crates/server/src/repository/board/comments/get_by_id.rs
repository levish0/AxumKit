use entity::board_comments::{Entity as CommentEntity, Model as CommentModel};
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};
use uuid::Uuid;

pub async fn repository_get_board_comment_by_id<C>(
    conn: &C,
    id: Uuid,
) -> Result<CommentModel, Errors>
where
    C: ConnectionTrait,
{
    let comment = CommentEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardCommentNotFound)?;

    Ok(comment)
}

/// Fetch a single board comment by ID with a row lock (`SELECT ... FOR UPDATE`).
///
/// Used to serialize concurrent deletes of the same comment so the post's
/// `comment_count` and the thread root's `reply_count` are adjusted exactly once.
///
/// # Errors
/// - `Errors::BoardCommentNotFound` if the comment does not exist.
/// - DB/storage error on query failure.
pub async fn repository_get_board_comment_by_id_for_update<C>(
    conn: &C,
    id: Uuid,
) -> Result<CommentModel, Errors>
where
    C: ConnectionTrait,
{
    let comment = CommentEntity::find_by_id(id)
        .lock_exclusive()
        .one(conn)
        .await?
        .ok_or(Errors::BoardCommentNotFound)?;

    Ok(comment)
}
