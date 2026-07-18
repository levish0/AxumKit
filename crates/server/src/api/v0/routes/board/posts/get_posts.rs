use crate::extractors::OptionalSession;
use crate::service::board::service_list_board_posts;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardPostListResponse, GetBoardPostsRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board/post/list",
    summary = "List board posts",
    description = "Returns posts from the requested board, applying filters and pagination parameters.",
    params(GetBoardPostsRequest),
    responses(
        (status = 200, description = "Post list retrieved successfully", body = BoardPostListResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn get_posts(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardPostsRequest>,
) -> Result<BoardPostListResponse, Errors> {
    service_list_board_posts(&state.db, payload, session.as_ref()).await
}
