use crate::extractors::OptionalSession;
use crate::service::board::service_list_board_comments;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardCommentListResponse, GetBoardCommentsRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board/comment/list",
    summary = "List board comments",
    description = "Returns a post's top-level comments, or the replies under a comment when parent_comment_id is set, with pagination.",
    params(GetBoardCommentsRequest),
    responses(
        (status = 200, description = "Comment list retrieved successfully", body = BoardCommentListResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Board Comments"
)]
pub async fn get_comments(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardCommentsRequest>,
) -> Result<BoardCommentListResponse, Errors> {
    service_list_board_comments(&state.db, payload, session.as_ref()).await
}
