use crate::extractors::OptionalSession;
use crate::service::board::service_get_board_by_slug;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardResponse, GetBoardBySlugRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/board/by-slug",
    summary = "Get a board by slug",
    description = "Returns the board with the given slug and the metadata visible to the caller.",
    params(GetBoardBySlugRequest),
    responses(
        (status = 200, description = "Board retrieved successfully", body = BoardResponse),
        (status = 404, description = "Board not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Boards"
)]
pub async fn get_board_by_slug(
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardBySlugRequest>,
) -> Result<BoardResponse, Errors> {
    service_get_board_by_slug(&state.db, &payload.slug, session.as_ref()).await
}
