use crate::repository::common::repository_query_exists;
use entity::board_comments::{Column as CommentColumn, Entity as CommentEntity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Whether a newer comment (larger uuidv7 id) exists in the same thread scope.
pub async fn repository_exists_newer_board_comment<C>(
    conn: &C,
    post_id: Uuid,
    parent_comment_id: Option<Uuid>,
    cursor_id: Uuid,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let mut query = CommentEntity::find()
        .filter(CommentColumn::PostId.eq(post_id))
        .filter(CommentColumn::Id.gt(cursor_id));

    query = match parent_comment_id {
        Some(pid) => query.filter(CommentColumn::ParentCommentId.eq(pid)),
        None => query.filter(CommentColumn::ParentCommentId.is_null()),
    };

    repository_query_exists(conn, query).await
}
