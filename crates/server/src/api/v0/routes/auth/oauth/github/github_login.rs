use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::github::service_github_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::Extension;
use axum::http::HeaderMap;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::oauth::request::github::GithubLoginRequest;
use dto::oauth::response::{OAuthPendingSignupResponse, OAuthSignInResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::Errors;
use std::net::SocketAddr;

///
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/github/login",
    request_body = GithubLoginRequest,
    responses(
        (status = 200, description = "New user - pending signup required", body = OAuthPendingSignupResponse),
        (status = 204, description = "Login successful (existing user)"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, invalid/expired state/code, or no verified email"),
        (status = 409, description = "Conflict - Email already exists"),
        (status = 500, description = "Internal Server Error - Database, Redis, or OAuth provider error")
    ),
    tag = "Auth"
)]
pub async fn auth_github_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<GithubLoginRequest>,
) -> Result<Response, Errors> {
    let user_agent_str = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let result = service_github_sign_in(
        &state.db,
        &state.redis_session,
        &state.http_client,
        &payload.code,
        &payload.state,
        &anonymous.anonymous_user_id,
        Some(user_agent_str),
        Some(ip_address),
    )
    .await?;

    OAuthSignInResponse::from_result(result).into_response_result()
}
