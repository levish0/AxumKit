use crate::extractors::RequiredSession;
use crate::service::board::service_reorder_board_pins;
use crate::state::AppState;
use axum::extract::State;
use dto::board::{BoardPostReorderPinsRequest, BoardPostReorderPinsResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/board/post/reorder-pins",
    summary = "Reorder a board's pinned posts",
    description = "Rewrites the display order of a board's pinned posts. `post_ids` is \
                   the full pin list, top first; positions are derived from the list \
                   index. The list must name exactly the board's current pin set — a \
                   stale one is rejected (409) rather than partially applied. Reordering \
                   neither pins nor unpins. Requires board moderation permission.",
    request_body = BoardPostReorderPinsRequest,
    responses(
        (status = 200, description = "Pins reordered successfully", body = BoardPostReorderPinsResponse),
        (status = 400, description = "Bad Request - Validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Board not found", body = ErrorResponse),
        (status = 409, description = "Conflict - The pin set changed since it was read", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn reorder_pins(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<BoardPostReorderPinsRequest>,
) -> Result<BoardPostReorderPinsResponse, Errors> {
    service_reorder_board_pins(&state.db, payload, &session).await
}
