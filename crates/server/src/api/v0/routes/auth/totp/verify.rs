use crate::service::auth::totp::{TotpVerifyResult, service_totp_verify};
use crate::state::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use dto::auth::request::TotpVerifyRequest;
use dto::auth::response::DeviceVerificationRequiredResponse;
use dto::auth::response::SessionTokenResponse;
use dto::auth::response::create_login_response;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/totp/verify",
    request_body = TotpVerifyRequest,
    responses(
        (status = 204, description = "TOTP verified, login successful"),
        (status = 202, description = "New-device email verification required", body = DeviceVerificationRequiredResponse),
        (status = 400, description = "Invalid TOTP code or temp token"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_verify(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<TotpVerifyRequest>,
) -> Result<Response, Errors> {
    let result = service_totp_verify(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.temp_token,
        &payload.code,
    )
    .await?;

    match result {
        TotpVerifyResult::SessionCreated {
            session_id,
            remember_me,
        } => create_login_response(session_id, remember_me),
        TotpVerifyResult::DeviceVerificationRequired => {
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}

#[utoipa::path(
    post,
    path = "/v0/app/auth/totp/verify",
    summary = "Finish login with TOTP (native-app client)",
    description = "Native-app variant of POST /v0/auth/totp/verify. Consumes the temporary token returned by POST /v0/app/auth/login, verifies a 6-digit authenticator code or an 8-character backup code, and returns the opaque session token in the response body — for replay as `Authorization: Bearer <token>` — instead of a cookie. Backup codes are single-use and are removed after successful verification.",
    request_body = TotpVerifyRequest,
    responses(
        (status = 200, description = "TOTP verification succeeded; the session token is returned in the body", body = SessionTokenResponse),
        (status = 400, description = "Malformed JSON payload, validation error, invalid or expired temporary token, or invalid verification code", body = ErrorResponse),
        (status = 401, description = "A backup code was submitted but no backup codes remain for this account", body = ErrorResponse),
        (status = 500, description = "Unexpected database or session store error", body = ErrorResponse)
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_verify_app(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<TotpVerifyRequest>,
) -> Result<Response, Errors> {
    let result = service_totp_verify(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.temp_token,
        &payload.code,
    )
    .await?;

    match result {
        // App client holds the token itself → return it in the body (no cookie).
        TotpVerifyResult::SessionCreated { session_id, .. } => {
            Ok(SessionTokenResponse::new(session_id).into_response())
        }
        // Unreachable for the app flow (temp token carries apply_device_check=false), but handle
        // exhaustively.
        TotpVerifyResult::DeviceVerificationRequired => {
            Ok(DeviceVerificationRequiredResponse::new().into_response())
        }
    }
}
