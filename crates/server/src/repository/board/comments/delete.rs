use entity::board_comments::Entity as CommentEntity;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, ModelTrait};
use uuid::Uuid;

/// Delete a board comment. Deleting a top-level comment cascades to its replies
/// (FK `on_delete = Cascade`).
pub async fn repository_delete_board_comment<C>(conn: &C, id: Uuid) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let comment = CommentEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardCommentNotFound)?;

    comment.delete(conn).await?;

    Ok(())
}
