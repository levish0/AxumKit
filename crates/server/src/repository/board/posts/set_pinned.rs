use entity::board_posts::{
    ActiveModel as PostActiveModel, Column as PostColumn, Entity as PostEntity, Model as PostModel,
};
use errors::errors::Errors;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

/// Pin or unpin a board post. Moderation-only (gated in the service layer).
///
/// Pinning takes the top slot as `min(pinned_position) - 1`, which is one write
/// and leaves every sibling untouched; unpinning just clears the column and
/// leaves a gap. Positions are therefore sparse and may go negative — they are a
/// relative sort key, never a dense sequence, and a reorder renormalizes them to
/// `0..n`. Nothing stops two concurrent pins from computing the same `min - 1`,
/// which is why the column is not unique: a tie is a stable `id` tiebreak on
/// read, whereas a unique index would make one moderator's pin fail outright.
pub async fn repository_set_board_post_pinned<C>(
    conn: &C,
    id: Uuid,
    pinned: bool,
) -> Result<PostModel, Errors>
where
    C: ConnectionTrait,
{
    let post = PostEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardPostNotFound)?;

    let position = if pinned {
        let top = PostEntity::find()
            .filter(PostColumn::BoardId.eq(post.board_id))
            .filter(PostColumn::PinnedPosition.is_not_null())
            .order_by_asc(PostColumn::PinnedPosition)
            .one(conn)
            .await?;

        Some(
            top.and_then(|top| top.pinned_position)
                .map_or(0, |min| min.saturating_sub(1)),
        )
    } else {
        None
    };

    let mut active: PostActiveModel = post.into();
    active.pinned_position = Set(position);
    Ok(active.update(conn).await?)
}
