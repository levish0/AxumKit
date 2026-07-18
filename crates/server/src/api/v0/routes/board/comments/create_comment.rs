use crate::extractors::RequiredSession;
use crate::service::board::service_create_board_comment;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use dto::board::{CreateBoardCommentRequest, CreateBoardCommentResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    post,
    path = "/v0/board/comment",
    summary = "Create a board comment",
    description = "Creates a comment or reply on the target post and returns the created comment.",
    request_body = CreateBoardCommentRequest,
    responses(
        (status = 201, description = "Comment created successfully", body = CreateBoardCommentResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions or post locked", body = ErrorResponse),
        (status = 404, description = "Post or parent comment not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    tag = "Board Comments"
)]
pub async fn create_comment(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<CreateBoardCommentRequest>,
) -> Result<CreateBoardCommentResponse, Errors> {
    let ip_address = extract_ip_address(&headers, addr);

    service_create_board_comment(&state.db, payload, &session, &ip_address).await
}
