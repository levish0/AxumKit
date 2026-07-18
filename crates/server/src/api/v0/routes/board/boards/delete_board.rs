use crate::extractors::RequiredSession;
use crate::service::board::service_delete_board;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{DeleteBoardRequest, DeleteBoardResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/delete",
    summary = "Delete a board",
    description = "Deletes the requested board when the caller has permission to remove it.",
    request_body = DeleteBoardRequest,
    responses(
        (status = 200, description = "Board deleted successfully", body = DeleteBoardResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Board not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn delete_board(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteBoardRequest>,
) -> Result<DeleteBoardResponse, Errors> {
    service_delete_board(&state.db, payload.board_id, &session).await
}
