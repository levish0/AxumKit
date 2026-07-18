use crate::extractors::RequiredSession;
use crate::service::board::service_create_board;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{CreateBoardRequest, CreateBoardResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board",
    summary = "Create a board",
    description = "Creates a new board from the submitted metadata and returns the created board.",
    request_body = CreateBoardRequest,
    responses(
        (status = 201, description = "Board created successfully", body = CreateBoardResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn create_board(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<CreateBoardRequest>,
) -> Result<CreateBoardResponse, Errors> {
    service_create_board(&state.db, payload, &session).await
}
