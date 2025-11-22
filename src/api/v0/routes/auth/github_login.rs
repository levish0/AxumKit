use crate::dto::auth::create_login_response;
use crate::dto::oauth::request::github::GithubLoginRequest;
use crate::errors::errors::Errors;
use crate::service::oauth::github_sign_in::service_github_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use crate::utils::validator::json_validator::ValidatedJson;
use axum::http::HeaderMap;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use std::net::SocketAddr;

/// GitHub OAuth 로그인을 처리합니다.
#[utoipa::path(
    post,
    path = "/v0/auth/github",
    request_body = GithubLoginRequest,
    responses(
        (status = 204, description = "Login successful"),
        (status = 400, description = "Invalid state or code"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Auth"
)]
pub async fn auth_github_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<GithubLoginRequest>,
) -> Result<Response, Errors> {
    let user_agent_str = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);
    // GitHub OAuth 로그인 처리 (세션 생성 포함)
    let session_id = service_github_sign_in(
        &state.conn,
        &state.redis_client,
        &state.http_client,
        &payload.code,
        &payload.state,
        payload.handle,
        Some(user_agent_str),
        Some(ip_address),
    )
    .await?;

    // 쿠키 설정하는 204 응답 반환
    create_login_response(session_id)
}
