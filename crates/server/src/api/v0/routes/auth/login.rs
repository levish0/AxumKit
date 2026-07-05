use crate::service::auth::LoginResult;
use crate::service::auth::device::DeviceCheck;
use crate::service::auth::login::service_login;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use crate::utils::extract::extract_user_agent::extract_user_agent;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{
    extract::{ConnectInfo, State},
    response::Response,
};
use axum_extra::{TypedHeader, headers::UserAgent};
use dto::auth::request::LoginRequest;
use dto::auth::response::create_login_response;
use dto::auth::response::{
    DeviceVerificationRequiredResponse, SessionTokenResponse, TotpRequiredResponse,
    device_cookie_name,
};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;
use tower_cookies::Cookies;

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 204, description = "Login successful"),
        (status = 202, description = "TOTP required", body = TotpRequiredResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error"),
        (status = 401, description = "Unauthorized - Invalid credentials or password not set"),
        (status = 404, description = "Not Found - User not found"),
        (status = 500, description = "Internal Server Error - Database or Redis error")
    ),
    tag = "Auth"
)]
pub async fn auth_login(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    cookies: Cookies,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);
    // Browser flow: present the device cookie (if any) for new-device verification.
    let device_token = cookies
        .get(&device_cookie_name())
        .map(|c| c.value().to_string());

    let result = service_login(
        &state.db,
        &state.redis_session,
        &state.worker,
        payload,
        Some(user_agent),
        Some(ip_address),
        DeviceCheck::Browser(device_token),
    )
    .await?;

    match result {
        LoginResult::SessionCreated {
            session_id,
            remember_me,
        } => create_login_response(session_id, remember_me),
        LoginResult::TotpRequired(temp_token) => {
            Ok(TotpRequiredResponse { temp_token }.into_response())
        }
        LoginResult::DeviceVerificationRequired => {
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}

#[utoipa::path(
    post,
    path = "/v0/app/auth/login",
    summary = "Authenticate with email and password (native-app client)",
    description = "Native-app variant of POST /v0/auth/login. On success the opaque session token is returned in the response body — for replay as `Authorization: Bearer <token>` — instead of an HttpOnly cookie, because app clients have no cookie jar. The TOTP branch is identical to the browser flow (202 + temporary token for POST /v0/app/auth/totp/verify). `remember_me` is ignored: there is no cookie to persist, and the server's sliding/absolute session lifetime applies regardless.",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login succeeded; the session token is returned in the body", body = SessionTokenResponse),
        (status = 202, description = "Primary credentials were accepted and TOTP verification is required", body = TotpRequiredResponse),
        (status = 400, description = "Malformed JSON payload or validation error", body = ErrorResponse),
        (status = 401, description = "Invalid credentials or this account cannot use password login", body = ErrorResponse),
        (status = 500, description = "Unexpected database or session store error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_login_app(
    user_agent: Option<TypedHeader<UserAgent>>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Response, Errors> {
    let user_agent = extract_user_agent(user_agent);
    let ip_address = extract_ip_address(&headers, addr);

    // App flow: no browser cookie jar, so new-device cookie verification does not apply.
    let result = service_login(
        &state.db,
        &state.redis_session,
        &state.worker,
        payload,
        Some(user_agent),
        Some(ip_address),
        DeviceCheck::Skip,
    )
    .await?;

    match result {
        LoginResult::SessionCreated { session_id, .. } => {
            // App client holds the token itself → return it in the body (no cookie).
            Ok(SessionTokenResponse::new(session_id).into_response())
        }
        LoginResult::TotpRequired(temp_token) => {
            Ok(TotpRequiredResponse { temp_token }.into_response())
        }
        LoginResult::DeviceVerificationRequired => {
            // Unreachable for the app flow (DeviceCheck::Skip), but handle exhaustively.
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}
