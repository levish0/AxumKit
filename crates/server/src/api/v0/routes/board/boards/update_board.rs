use crate::extractors::RequiredSession;
use crate::service::board::service_update_board;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{UpdateBoardRequest, UpdateBoardResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/update",
    summary = "Update a board",
    description = "Updates the requested board with the submitted changes and returns the updated board.",
    request_body = UpdateBoardRequest,
    responses(
        (status = 200, description = "Board updated successfully", body = UpdateBoardResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Board not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn update_board(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateBoardRequest>,
) -> Result<UpdateBoardResponse, Errors> {
    service_update_board(&state.db, payload, &session).await
}
