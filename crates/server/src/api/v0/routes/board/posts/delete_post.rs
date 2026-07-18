use crate::extractors::RequiredSession;
use crate::service::board::service_delete_board_post;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{DeleteBoardPostRequest, DeleteBoardPostResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/post/delete",
    summary = "Delete a board post",
    description = "Deletes the requested board post when the caller has permission to remove it.",
    request_body = DeleteBoardPostRequest,
    responses(
        (status = 200, description = "Post deleted successfully", body = DeleteBoardPostResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn delete_post(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteBoardPostRequest>,
) -> Result<DeleteBoardPostResponse, Errors> {
    service_delete_board_post(&state.db, payload.post_id, &session).await
}
