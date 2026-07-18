use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::posts::{
    repository_get_board_post_by_id, repository_set_board_post_locked,
    repository_set_board_post_pinned,
};
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use constants::ModerationAction;
use dto::board::{BoardPostModerationRequest, BoardPostModerationResponse};
use entity::common::ModerationResourceType;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// The four board post moderation operations, all gated by `BoardPermission::Moderate`.
#[derive(Clone, Copy)]
enum BoardPostOp {
    Pin,
    Unpin,
    Lock,
    Unlock,
}

impl BoardPostOp {
    fn action(self) -> ModerationAction {
        match self {
            BoardPostOp::Pin => ModerationAction::BoardPin,
            BoardPostOp::Unpin => ModerationAction::BoardUnpin,
            BoardPostOp::Lock => ModerationAction::BoardLock,
            BoardPostOp::Unlock => ModerationAction::BoardUnlock,
        }
    }
}

/// Shared implementation: gate on board moderation, flip the flag, log the action.
/// Board roles are global, so any `ModBoard`/`Admin` may moderate any board's posts —
/// no per-board scoping or board load is required beyond the post's `board_id`.
async fn moderate_board_post(
    db: &DatabaseConnection,
    op: BoardPostOp,
    payload: BoardPostModerationRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostModerationResponse> {
    let post = repository_get_board_post_by_id(db, payload.post_id).await?;

    let ctx = PermissionService::get_context(db, Some(session)).await?;
    BoardPermission::Moderate.check(&ctx)?;

    let txn = db.begin().await?;

    let updated = match op {
        BoardPostOp::Pin => repository_set_board_post_pinned(&txn, payload.post_id, true).await?,
        BoardPostOp::Unpin => {
            repository_set_board_post_pinned(&txn, payload.post_id, false).await?
        }
        BoardPostOp::Lock => repository_set_board_post_locked(&txn, payload.post_id, true).await?,
        BoardPostOp::Unlock => {
            repository_set_board_post_locked(&txn, payload.post_id, false).await?
        }
    };

    repository_create_moderation_log(
        &txn,
        op.action(),
        Some(session.user_id),
        ModerationResourceType::BoardPost,
        Some(payload.post_id),
        payload.reason,
        Some(json!({ "board_id": post.board_id })),
    )
    .await?;

    txn.commit().await?;

    info!(post_id = %updated.id, action = %op.action(), "Board post moderated");

    Ok(BoardPostModerationResponse {
        post_id: updated.id,
        is_pinned: updated.pinned_position.is_some(),
        is_locked: updated.is_locked,
    })
}

pub async fn service_pin_board_post(
    db: &DatabaseConnection,
    payload: BoardPostModerationRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostModerationResponse> {
    moderate_board_post(db, BoardPostOp::Pin, payload, session).await
}

pub async fn service_unpin_board_post(
    db: &DatabaseConnection,
    payload: BoardPostModerationRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostModerationResponse> {
    moderate_board_post(db, BoardPostOp::Unpin, payload, session).await
}

pub async fn service_lock_board_post(
    db: &DatabaseConnection,
    payload: BoardPostModerationRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostModerationResponse> {
    moderate_board_post(db, BoardPostOp::Lock, payload, session).await
}

pub async fn service_unlock_board_post(
    db: &DatabaseConnection,
    payload: BoardPostModerationRequest,
    session: &SessionContext,
) -> ServiceResult<BoardPostModerationResponse> {
    moderate_board_post(db, BoardPostOp::Unlock, payload, session).await
}
