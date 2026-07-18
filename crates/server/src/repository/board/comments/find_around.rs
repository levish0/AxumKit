use entity::board_comments::{
    Column as CommentColumn, Entity as CommentEntity, Model as CommentModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// The page window *containing* `focus` — the anchor for a comment deep-link.
///
/// Returns up to `limit` comments from the same listing scope (`post_id` +
/// `parent_comment_id`), ascending, with `focus` inside. The window is centered
/// on the focus when both sides have enough comments, and shifts toward the
/// other side near a thread edge, so a full page is returned whenever the
/// thread has one.
///
/// The caller is responsible for having verified that `focus` belongs to the
/// requested scope.
pub async fn repository_find_board_comments_around<C>(
    conn: &C,
    post_id: Uuid,
    parent_comment_id: Option<Uuid>,
    focus: CommentModel,
    limit: u64,
) -> Result<Vec<CommentModel>, Errors>
where
    C: ConnectionTrait,
{
    let scoped = || {
        let query = CommentEntity::find().filter(CommentColumn::PostId.eq(post_id));
        match parent_comment_id {
            Some(pid) => query.filter(CommentColumn::ParentCommentId.eq(pid)),
            None => query.filter(CommentColumn::ParentCommentId.is_null()),
        }
    };

    // Fetch up to a full page on each side; the assembly below decides how much
    // of each to keep. Ids are UUIDv7 (time-ordered), like the cursor listing.
    let neighbors = limit.saturating_sub(1) as usize;
    let mut older = scoped()
        .filter(CommentColumn::Id.lt(focus.id))
        .order_by_desc(CommentColumn::Id)
        .limit(neighbors as u64)
        .all(conn)
        .await?;
    let newer = scoped()
        .filter(CommentColumn::Id.gt(focus.id))
        .order_by_asc(CommentColumn::Id)
        .limit(neighbors as u64)
        .all(conn)
        .await?;

    // Center the focus, then let either side absorb the slack the other side
    // cannot fill (focus near the thread start/end).
    let take_older = (neighbors / 2)
        .max(neighbors.saturating_sub(newer.len()))
        .min(older.len());
    let take_newer = (neighbors - take_older).min(newer.len());

    older.truncate(take_older);
    older.reverse();

    let mut window = older;
    window.push(focus);
    window.extend(newer.into_iter().take(take_newer));

    Ok(window)
}
