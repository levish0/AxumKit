use crate::extractors::OptionalSession;
use crate::service::board::service_get_board;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardResponse, GetBoardRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board",
    summary = "Get a board",
    description = "Returns the requested board and the metadata visible to the caller.",
    params(GetBoardRequest),
    responses(
        (status = 200, description = "Board retrieved successfully", body = BoardResponse),
        (status = 404, description = "Board not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn get_board(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardRequest>,
) -> Result<BoardResponse, Errors> {
    service_get_board(&state.db, payload.board_id, session.as_ref()).await
}
