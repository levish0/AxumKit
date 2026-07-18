use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::repository_get_board_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts;
use crate::service::board::mapper::build_board_response;
use dto::board::BoardResponse;
use entity::boards::Model as BoardModel;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

pub async fn service_get_board(
    db: &DatabaseConnection,
    board_id: Uuid,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardResponse> {
    let board = repository_get_board_by_id(db, board_id).await?;
    authorize_view_and_map(db, board, session).await
}

/// Applies the board View gate for the caller and maps the entity to its response.
///
/// Shared by id and slug lookups so both expose identical visibility rules and
/// capability flags.
pub(super) async fn authorize_view_and_map(
    db: &DatabaseConnection,
    board: BoardModel,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardResponse> {
    let ctx = PermissionService::get_context(db, session).await?;
    let facts = load_board_facts(db, &board).await?;
    BoardPermission::View(facts.clone()).check(&ctx)?;

    Ok(build_board_response(&ctx, board, facts))
}
