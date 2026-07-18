use crate::service::auth::LoginResult;
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
    DEVICE_TOKEN_HEADER, DeviceVerificationRequiredResponse, SessionTokenResponse,
    TotpRequiredResponse, device_cookie_name,
};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};
use std::net::SocketAddr;
use tower_cookies::Cookies;

#[utoipa::path(
    post,
    path = "/v0/auth/login",
    summary = "Authenticate with email and password",
    description = "Validates the submitted credentials. If the account has TOTP enabled, this endpoint returns 202 Accepted with a temporary token for POST /v0/auth/totp/verify instead of creating a session. Otherwise it creates a session immediately and sets the session cookie.",
    request_body = LoginRequest,
    responses(
        (status = 204, description = "Login succeeded and a session cookie was issued"),
        (status = 202, description = "TOTP or new-device email verification is required", body = TotpRequiredResponse),
        (status = 400, description = "Malformed JSON payload or validation error", body = ErrorResponse),
        (status = 401, description = "Invalid credentials or this account cannot use password login", body = ErrorResponse),
        (status = 500, description = "Unexpected database or session store error", body = ErrorResponse)
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

    // Handle login
    let result = service_login(
        &state.db,
        &state.redis_session,
        &state.worker,
        payload,
        user_agent,
        Some(ip_address),
        device_token,
    )
    .await?;

    match result {
        LoginResult::SessionCreated {
            session_id,
            remember_me,
        } => {
            // Return a 204 response that sets the cookie
            create_login_response(session_id, remember_me)
        }
        LoginResult::TotpRequired(temp_token) => {
            // TOTP required: return 202 + temp_token
            Ok(TotpRequiredResponse { temp_token }.into_response())
        }
        LoginResult::DeviceVerificationRequired => {
            // New device: 202 without a session; a verification email has been sent.
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}

#[utoipa::path(
    post,
    path = "/v0/app/auth/login",
    summary = "Authenticate with email and password (native-app client)",
    description = "Native-app variant of POST /v0/auth/login. On success the opaque session token is returned in the response body — for replay as `Authorization: Bearer <token>` — instead of an HttpOnly cookie, because app clients have no cookie jar. New-device verification applies exactly as in the browser flow: the app presents its stored device-recognition token in the `X-Device-Token` header (the app-channel equivalent of the browser device cookie); an unrecognized (or absent) device returns 202 and emails a challenge to complete via POST /v0/app/auth/device/verify. The TOTP branch is identical to the browser flow (202 + temporary token for POST /v0/app/auth/totp/verify). `remember_me` is ignored: there is no cookie to persist, and the server's sliding/absolute session lifetime applies regardless.",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login succeeded; the session token is returned in the body", body = SessionTokenResponse),
        (status = 202, description = "TOTP or new-device email verification is required", body = TotpRequiredResponse),
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
    // App flow: the device-recognition token arrives in the `X-Device-Token` header (no cookie jar).
    // Same new-device gate as the browser — an unrecognized/absent token triggers an email challenge.
    let device_token = headers
        .get(DEVICE_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let result = service_login(
        &state.db,
        &state.redis_session,
        &state.worker,
        payload,
        user_agent,
        Some(ip_address),
        device_token,
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
            // Unrecognized device: session withheld, challenge emailed. The app completes it via
            // POST /v0/app/auth/device/verify (returns the session + device token in the body).
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}
