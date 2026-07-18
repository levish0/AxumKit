use crate::extractors::OptionalSession;
use crate::service::board::service_get_board_permissions;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardPermissionsResponse, GetBoardPermissionsRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board/permissions",
    summary = "Get board permissions",
    description = "Returns the effective permissions for the caller on the requested board.",
    operation_id = "getBoardPermissions",
    params(GetBoardPermissionsRequest),
    responses(
        (status = 200, description = "Board permissions retrieved successfully", body = BoardPermissionsResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error", body = ErrorResponse),
        (status = 404, description = "Not Found - Board not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn get_permissions(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardPermissionsRequest>,
) -> Result<BoardPermissionsResponse, Errors> {
    let permissions =
        service_get_board_permissions(&state.db, session.as_ref(), payload.board_id).await?;
    Ok(permissions)
}
