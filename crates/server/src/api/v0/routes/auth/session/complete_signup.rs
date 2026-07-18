use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::service::oauth::complete_signup::service_complete_signup;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::Extension;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::auth::request::CompleteSignupRequest;
use dto::auth::response::SessionTokenResponse;
use dto::auth::response::create_login_response;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;

/// Completes an OAuth pending signup.
///
/// For new users during OAuth login, submit the returned pending_token
/// along with a handle and display_name to finish signing up.
#[utoipa::path(
    post,
    path = "/v0/auth/complete-signup",
    summary = "Complete an OAuth signup after provider sign-in",
    description = "Consumes the pending signup token returned by an OAuth login for a new user, verifies that the token belongs to the same anonymous browser context, creates the user and OAuth connection, and signs the user in by setting the session cookie.",
    request_body = CompleteSignupRequest,
    responses(
        (status = 204, description = "Pending signup was completed and a session cookie was issued"),
        (status = 400, description = "Malformed JSON payload, validation error, or another completion attempt is already in progress", body = ErrorResponse),
        (status = 401, description = "Pending signup token is missing, expired, invalid, or bound to a different browser context", body = ErrorResponse),
        (status = 409, description = "The handle, email, or OAuth identity is already in use", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, storage, or OAuth-related error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_complete_signup(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(anonymous): Extension<AnonymousUserContext>,
    ValidatedJson(payload): ValidatedJson<CompleteSignupRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // Complete the OAuth pending signup
    let session_id = service_complete_signup(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.pending_token,
        &payload.handle,
        &payload.display_name,
        Some(anonymous.anonymous_user_id.as_str()),
        user_agent,
        Some(ip_address),
    )
    .await?;

    // Non-persistent session cookie (no remember-me in the OAuth signup flow);
    // the server still enforces the absolute session lifetime.
    create_login_response(session_id, false)
}

/// Complete an OAuth pending signup for a native-app client (provider-token flow).
///
/// Counterpart to [`auth_complete_signup`] for apps: the pending token came from
/// POST /v0/app/auth/oauth/{provider}/token and is bound only by its own secrecy (no anonymous
/// browser context), and the new session token is returned in the response body instead of a
/// cookie.
#[utoipa::path(
    post,
    path = "/v0/app/auth/complete-signup",
    summary = "Complete an OAuth signup from a native app",
    description = "Native-app variant of POST /v0/auth/complete-signup. Consumes the pending signup token returned by a native provider-token sign-in for a new user, creates the user and OAuth connection, and returns the opaque session token in the body for `Authorization: Bearer` use. The pending token is single-use and short-lived; unlike the browser flow there is no anonymous-cookie binding (the app has no cookie jar), so the token's secrecy is the binding.",
    request_body = CompleteSignupRequest,
    responses(
        (status = 200, description = "Pending signup completed; session token returned in the body", body = SessionTokenResponse),
        (status = 400, description = "Malformed JSON payload, validation error, or another completion attempt is already in progress", body = ErrorResponse),
        (status = 401, description = "Pending signup token is missing, expired, or invalid", body = ErrorResponse),
        (status = 409, description = "The handle, email, or OAuth identity is already in use", body = ErrorResponse),
        (status = 500, description = "Unexpected database, Redis, storage, or OAuth-related error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_complete_signup_app(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CompleteSignupRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    let raw_token = service_complete_signup(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.pending_token,
        &payload.handle,
        &payload.display_name,
        // Native app: no anonymous browser context → pending token's secrecy is the binding.
        None,
        user_agent,
        Some(ip_address),
    )
    .await?;

    // App client holds the token itself → return it in the body (no cookie).
    Ok(SessionTokenResponse::new(raw_token).into_response())
}
