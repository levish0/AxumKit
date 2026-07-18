use entity::board_posts::{Column as PostColumn, Entity as PostEntity, Model as PostModel};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

/// List a board's pinned posts in display order, top first.
///
/// Deliberately unbounded: pins sit outside pagination and every page of the
/// listing carries the whole set, so there is no page to limit them to. How many
/// pins a board should have is that board's moderation policy, not a schema cap
/// — and only moderators can pin, so the count cannot be driven by users.
///
/// The `id` tiebreak keeps the order total: `pinned_position` is not unique, so
/// two pins raced onto the same slot must still sort deterministically.
pub async fn repository_find_pinned_board_posts<C>(
    conn: &C,
    board_id: Uuid,
) -> Result<Vec<PostModel>, Errors>
where
    C: ConnectionTrait,
{
    let posts = PostEntity::find()
        .filter(PostColumn::BoardId.eq(board_id))
        .filter(PostColumn::PinnedPosition.is_not_null())
        .order_by_asc(PostColumn::PinnedPosition)
        .order_by_asc(PostColumn::Id)
        .all(conn)
        .await?;

    Ok(posts)
}
