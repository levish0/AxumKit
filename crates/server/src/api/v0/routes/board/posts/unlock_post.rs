use crate::extractors::RequiredSession;
use crate::service::board::service_unlock_board_post;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardPostModerationRequest, BoardPostModerationResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/post/unlock",
    summary = "Unlock a board post",
    description = "Removes the locked state from the post. Requires board moderation permission.",
    request_body = BoardPostModerationRequest,
    responses(
        (status = 200, description = "Post unlocked successfully", body = BoardPostModerationResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn unlock_post(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<BoardPostModerationRequest>,
) -> Result<BoardPostModerationResponse, Errors> {
    service_unlock_board_post(&state.db, payload, &session).await
}
