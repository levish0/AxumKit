use dto::pagination::CursorDirection;
use entity::board_comments::{
    Column as CommentColumn, Entity as CommentEntity, Model as CommentModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// List a post's top-level comments (`parent_comment_id IS NULL`) or the replies under a
/// specific top-level comment, with cursor-based pagination keyed on the uuidv7 id
/// (time-ordered). Defaults to oldest-first.
pub async fn repository_find_board_comments<C>(
    conn: &C,
    post_id: Uuid,
    parent_comment_id: Option<Uuid>,
    cursor_id: Option<Uuid>,
    cursor_direction: Option<CursorDirection>,
    limit: u64,
) -> Result<Vec<CommentModel>, Errors>
where
    C: ConnectionTrait,
{
    let mut query = CommentEntity::find().filter(CommentColumn::PostId.eq(post_id));

    query = match parent_comment_id {
        Some(pid) => query.filter(CommentColumn::ParentCommentId.eq(pid)),
        None => query.filter(CommentColumn::ParentCommentId.is_null()),
    };

    if let Some(id) = cursor_id {
        // This thread reads oldest-first, so a bare cursor (no direction) advances
        // forward toward newer comments, like discussion messages — not Older.
        let direction = cursor_direction.unwrap_or(CursorDirection::Newer);
        query = match direction {
            CursorDirection::Older => query
                .filter(CommentColumn::Id.lt(id))
                .order_by_desc(CommentColumn::Id),
            CursorDirection::Newer => query
                .filter(CommentColumn::Id.gt(id))
                .order_by_asc(CommentColumn::Id),
        };
    } else {
        // Default: oldest first (ascending), the natural reading order for a thread.
        query = query.order_by_asc(CommentColumn::Id);
    }

    let comments = query.limit(limit).all(conn).await?;

    Ok(comments)
}
