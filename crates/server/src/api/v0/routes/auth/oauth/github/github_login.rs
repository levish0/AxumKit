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
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

/// Handles GitHub OAuth login.
///
/// - Existing user: 204 No Content + Set-Cookie
/// - New user: 200 OK + pending signup info (complete-signup required)
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/github/login",
    summary = "Sign in with GitHub OAuth",
    description = "Exchanges the GitHub authorization code and validated state for provider identity. Existing linked accounts receive a session immediately. New identities receive a pending signup token that must be completed via POST /v0/auth/complete-signup. If the profile payload does not include an email, the service fetches verified emails from GitHub.",
    request_body = GithubLoginRequest,
    responses(
        (status = 200, description = "GitHub identity was accepted but profile completion is still required", body = OAuthPendingSignupResponse),
        (status = 204, description = "GitHub identity matched an existing account and a session cookie was issued"),
        (status = 400, description = "Malformed JSON payload, validation error, invalid or expired state or code, or GitHub did not provide a verified primary email", body = ErrorResponse),
        (status = 409, description = "A local account already uses the same email address", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, or GitHub OAuth error", body = ErrorResponse)
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
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // Handle GitHub OAuth login
    let result = service_github_sign_in(
        &state.db,
        &state.redis_session,
        &state.http_client,
        &payload.code,
        &payload.state,
        &anonymous.anonymous_user_id,
        user_agent,
        Some(ip_address),
    )
    .await?;

    // Convert SignInResult into an HTTP response
    OAuthSignInResponse::from_result(result).into_response_result()
}
