use crate::extractors::OptionalSession;
use crate::service::board::service_list_boards;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardListResponse, GetBoardsRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board/list",
    summary = "List boards",
    description = "Returns the boards visible to the caller, applying any submitted filters or pagination parameters.",
    params(GetBoardsRequest),
    responses(
        (status = 200, description = "Board list retrieved successfully", body = BoardListResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn get_boards(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardsRequest>,
) -> Result<BoardListResponse, Errors> {
    service_list_boards(&state.db, payload, session.as_ref()).await
}
