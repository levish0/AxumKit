use entity::board_posts::{Column as PostColumn, Entity as PostEntity, Model as PostModel};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// List a board's unpinned top-level posts, newest-first.
///
/// Pins are excluded rather than sorted to the front: they are read whole by
/// [`repository_find_pinned_board_posts`] and rendered above every page. Ordering
/// them first inside this offset window instead — as this query once did — both
/// stranded them on page 1 and spent that page's slots on them.
///
/// [`repository_find_pinned_board_posts`]: super::repository_find_pinned_board_posts
pub async fn repository_find_board_posts<C>(
    conn: &C,
    board_id: Uuid,
    offset: u64,
    limit: u64,
) -> Result<Vec<PostModel>, Errors>
where
    C: ConnectionTrait,
{
    let posts = PostEntity::find()
        .filter(PostColumn::BoardId.eq(board_id))
        .filter(PostColumn::PinnedPosition.is_null())
        .order_by_desc(PostColumn::CreatedAt)
        .order_by_desc(PostColumn::Id)
        .offset(offset)
        .limit(limit)
        .all(conn)
        .await?;

    Ok(posts)
}
