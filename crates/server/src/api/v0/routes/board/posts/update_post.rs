use crate::extractors::RequiredSession;
use crate::service::board::service_update_board_post;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use dto::board::{UpdateBoardPostRequest, UpdateBoardPostResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/board/post/update",
    summary = "Update a board post",
    description = "Updates the requested board post with the submitted changes and returns the updated post.",
    request_body = UpdateBoardPostRequest,
    responses(
        (status = 200, description = "Post updated successfully", body = UpdateBoardPostResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn update_post(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateBoardPostRequest>,
) -> Result<UpdateBoardPostResponse, Errors> {
    let ip_address = extract_ip_address(&headers, addr);

    service_update_board_post(&state.db, payload, &session, &ip_address).await
}
