use crate::extractors::OptionalSession;
use crate::service::board::service_get_board_post;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use dto::board::{BoardPostResponse, GetBoardPostRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

#[utoipa::path(
    get,
    path = "/v0/board/post",
    summary = "Get a board post",
    description = "Returns the requested board post and the metadata visible to the caller.",
    params(GetBoardPostRequest),
    responses(
        (status = 200, description = "Post retrieved successfully", body = BoardPostResponse),
        (status = 404, description = "Post not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    tag = "Board Posts"
)]
pub async fn get_post(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    OptionalSession(session): OptionalSession,
    ValidatedQuery(payload): ValidatedQuery<GetBoardPostRequest>,
) -> Result<BoardPostResponse, Errors> {
    let ip_address = extract_ip_address(&headers, addr);

    service_get_board_post(
        &state.db,
        &state.redis_cache,
        payload.post_id,
        session.as_ref(),
        &ip_address,
    )
    .await
}
