use crate::service::oauth::google_sign_in::service_google_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use axumkit_dto::oauth::request::google::GoogleLoginRequest;
use axumkit_dto::oauth::response::{OAuthPendingSignupResponse, OAuthSignInResponse};
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;
use std::net::SocketAddr;

/// Google OAuth 로그인을 처리합니다.
///
/// - 기존 사용자: 204 No Content + Set-Cookie
/// - 신규 사용자: 200 OK + pending signup 정보 (complete-signup 필요)
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/google/login",
    request_body = GoogleLoginRequest,
    responses(
        (status = 200, description = "New user - pending signup required", body = OAuthPendingSignupResponse),
        (status = 204, description = "Login successful (existing user)"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, or invalid/expired state/code"),
        (status = 409, description = "Conflict - Email already exists"),
        (status = 500, description = "Internal Server Error - Database, Redis, or OAuth provider error")
    ),
    tag = "Auth"
)]
pub async fn auth_google_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<GoogleLoginRequest>,
) -> Result<Response, Errors> {
    let user_agent_str = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // Google OAuth 로그인 처리
    let result = service_google_sign_in(
        &state.conn,
        &state.redis_session,
        &state.http_client,
        &payload.code,
        &payload.state,
        Some(user_agent_str),
        Some(ip_address),
    )
    .await?;

    // SignInResult를 HTTP 응답으로 변환
    OAuthSignInResponse::from_result(result).into_response_result()
}
