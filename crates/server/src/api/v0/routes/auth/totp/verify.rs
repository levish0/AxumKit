use crate::service::auth::totp::service_totp_verify;
use crate::state::AppState;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use dto::auth::request::TotpVerifyRequest;
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
        &payload.temp_token,
        &payload.code,
    )
    .await?;

    create_login_response(result.session_id, result.remember_me)
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
        &payload.temp_token,
        &payload.code,
    )
    .await?;

    // App client holds the token itself → return it in the body (no cookie).
    Ok(SessionTokenResponse::new(result.session_id).into_response())
}
