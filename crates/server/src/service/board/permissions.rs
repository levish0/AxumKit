use crate::permission::board::{BoardFacts, BoardPermission};
use crate::permission::rule::Rule;
use crate::permission::{PermissionService, UserContext};
use crate::repository::board::repository_get_board_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use dto::board::BoardPermissionsResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

pub async fn service_get_board_permissions(
    db: &DatabaseConnection,
    session: Option<&SessionContext>,
    board_id: Uuid,
) -> ServiceResult<BoardPermissionsResponse> {
    let board = repository_get_board_by_id(db, board_id).await?;

    // Load the caller context once and derive every flag from the board rules.
    let ctx = PermissionService::get_context(db, session).await?;
    let facts = load_board_facts(db, &board).await?;

    Ok(build_board_permissions_response(&ctx, facts))
}

/// Single source of truth for board-level capability flags. The board GET
/// mapper reuses this so the embedded `can_write`/`can_moderate` flags can
/// never drift from the standalone permissions endpoint.
pub(crate) fn build_board_permissions_response(
    ctx: &UserContext,
    facts: BoardFacts,
) -> BoardPermissionsResponse {
    BoardPermissionsResponse {
        can_view: BoardPermission::View(facts.clone()).is_allowed(ctx),
        can_write: BoardPermission::Write(facts).is_allowed(ctx),
        can_moderate: BoardPermission::Moderate.is_allowed(ctx),
        can_manage: BoardPermission::ManageBoard.is_allowed(ctx),
    }
}
