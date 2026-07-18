use crate::extractors::RequiredSession;
use crate::service::board::service_update_board_comment;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use dto::board::{UpdateBoardCommentRequest, UpdateBoardCommentResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/board/comment/update",
    summary = "Update a board comment",
    description = "Updates the requested board comment with the submitted content and returns the updated comment.",
    request_body = UpdateBoardCommentRequest,
    responses(
        (status = 200, description = "Comment updated successfully", body = UpdateBoardCommentResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Comment not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Comments"
)]
pub async fn update_comment(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateBoardCommentRequest>,
) -> Result<UpdateBoardCommentResponse, Errors> {
    let ip_address = extract_ip_address(&headers, addr);

    service_update_board_comment(&state.db, payload, &session, &ip_address).await
}
