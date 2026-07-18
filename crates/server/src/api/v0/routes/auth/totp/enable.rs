use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_enable;
use crate::state::AppState;
use axum::extract::State;
use dto::auth::request::TotpEnableRequest;
use dto::auth::response::TotpEnableResponse;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/totp/enable",
    summary = "Enable TOTP for the current account",
    description = "Verifies the first code from the authenticator app against the secret created by the setup step. On success, marks TOTP as enabled and returns a new set of backup codes.",
    request_body = TotpEnableRequest,
    responses(
        (status = 200, description = "TOTP was enabled and backup codes were generated", body = TotpEnableResponse),
        (status = 400, description = "Malformed JSON payload, validation error, setup was not started, or the TOTP code is invalid", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 409, description = "TOTP is already enabled for this account", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_enable(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<TotpEnableRequest>,
) -> Result<TotpEnableResponse, Errors> {
    service_totp_enable(&state.db, session.user_id, &payload.code).await
}
