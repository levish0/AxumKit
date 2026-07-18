use crate::extractors::RequiredSession;
use crate::service::board::service_create_board_post;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use dto::board::{CreateBoardPostRequest, CreateBoardPostResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/board/post",
    summary = "Create a board post",
    description = "Creates a new post in the target board and returns the created post.",
    request_body = CreateBoardPostRequest,
    responses(
        (status = 201, description = "Post created successfully", body = CreateBoardPostResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn create_post(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<CreateBoardPostRequest>,
) -> Result<CreateBoardPostResponse, Errors> {
    let ip_address = extract_ip_address(&headers, addr);

    service_create_board_post(&state.db, payload, &session, &ip_address).await
}
