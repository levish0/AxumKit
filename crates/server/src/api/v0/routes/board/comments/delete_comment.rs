use crate::extractors::RequiredSession;
use crate::service::board::service_delete_board_comment;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{DeleteBoardCommentRequest, DeleteBoardCommentResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/comment/delete",
    summary = "Delete a board comment",
    description = "Deletes the requested board comment when the caller has permission to remove it.",
    request_body = DeleteBoardCommentRequest,
    responses(
        (status = 200, description = "Comment deleted successfully", body = DeleteBoardCommentResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Comment not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Comments"
)]
pub async fn delete_comment(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteBoardCommentRequest>,
) -> Result<DeleteBoardCommentResponse, Errors> {
    service_delete_board_comment(&state.db, payload.comment_id, &session).await
}
