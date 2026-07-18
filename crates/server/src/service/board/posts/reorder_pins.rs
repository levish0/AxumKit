use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::posts::{
    repository_find_pinned_board_posts, repository_reorder_board_pins,
};
use crate::repository::board::repository_get_board_by_id;
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use constants::ModerationAction;
use dto::board::{BoardPostReorderPinsRequest, BoardPostReorderPinsResponse};
use entity::common::ModerationResourceType;
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use std::collections::HashSet;
use tracing::info;

/// Rewrite a board's pin display order from a moderator-supplied list.
///
/// Gated by `BoardPermission::Moderate`, exactly like pin/unpin — board roles are
/// global, so this needs no per-board ACL scoping. Anything stricter would leave
/// a moderator able to pin a post but not place it.
///
/// `post_ids` must name exactly the board's current pin set. A list that omits a
/// pin or names an unpinned post comes from a moderator who has not seen a
/// concurrent pin/unpin; applying it would silently drop or resurrect a pin they
/// never saw, so it is rejected whole. This makes the payload its own optimistic
/// concurrency token: re-read, then retry.
pub async fn service_reorder_board_pins(
    db: &DatabaseConnection,
    payload: BoardPostReorderPinsRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostReorderPinsResponse> {
    let board = repository_get_board_by_id(db, payload.board_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;
    BoardPermission::Moderate.check(&ctx)?;

    let txn = db.begin().await?;

    let current: HashSet<_> = repository_find_pinned_board_posts(&txn, board.id)
        .await?
        .into_iter()
        .map(|post| post.id)
        .collect();
    let requested: HashSet<_> = payload.post_ids.iter().copied().collect();

    // A duplicate id would pass the set comparison while silently shortening the
    // list, so the length check is load-bearing, not a formality.
    if requested.len() != payload.post_ids.len() || requested != current {
        return Err(Errors::BoardPinSetMismatch);
    }

    repository_reorder_board_pins(&txn, &payload.post_ids).await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::BoardReorderPins,
        Some(session.user_id),
        ModerationResourceType::Board,
        Some(board.id),
        payload.reason,
        Some(json!({ "post_ids": &payload.post_ids })),
    )
    .await?;

    txn.commit().await?;

    info!(
        board_id = %board.id,
        pin_count = payload.post_ids.len(),
        action = %ModerationAction::BoardReorderPins,
        "Board pins reordered"
    );

    Ok(BoardPostReorderPinsResponse {
        board_id: board.id,
        post_ids: payload.post_ids,
    })
}
