use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::google::service_google_sign_in;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::Extension;
use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::oauth::request::google::GoogleLoginRequest;
use dto::oauth::response::{OAuthPendingSignupResponse, OAuthSignInResponse};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

/// Handles Google OAuth login.
///
/// - Existing user: 204 No Content + Set-Cookie
/// - New user: 200 OK + pending signup info (complete-signup required)
#[utoipa::path(
    post,
    path = "/v0/auth/oauth/google/login",
    summary = "Sign in with Google OAuth",
    description = "Exchanges the Google authorization code and validated state for provider identity. If the Google account is already linked, this endpoint creates a session immediately. Otherwise it stores pending signup data in Redis and returns a token that must be completed via POST /v0/auth/complete-signup.",
    request_body = GoogleLoginRequest,
    responses(
        (status = 200, description = "Google identity was accepted but profile completion is still required", body = OAuthPendingSignupResponse),
        (status = 204, description = "Google identity matched an existing account and a session cookie was issued"),
        (status = 400, description = "Malformed JSON payload, validation error, invalid or expired state or code, or the Google account email is not verified", body = ErrorResponse),
        (status = 409, description = "A local account already uses the same email address", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, or Google OAuth error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_google_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<GoogleLoginRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // Handle Google OAuth login
    let result = service_google_sign_in(
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
