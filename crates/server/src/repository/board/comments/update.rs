use chrono::Utc;
use entity::board_comments::{
    ActiveModel as CommentActiveModel, Entity as CommentEntity, Model as CommentModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

pub async fn repository_update_board_comment<C>(
    conn: &C,
    id: Uuid,
    content: String,
) -> Result<CommentModel, Errors>
where
    C: ConnectionTrait,
{
    let comment = CommentEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardCommentNotFound)?;

    let mut active: CommentActiveModel = comment.into();
    active.content = Set(content);
    active.edited_at = Set(Some(Utc::now()));

    let updated = active.update(conn).await?;
    Ok(updated)
}
