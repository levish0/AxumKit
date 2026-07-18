use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_disable;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use dto::auth::request::TotpDisableRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/totp/disable",
    summary = "Disable TOTP for the current account",
    description = "Requires a current 6-digit TOTP code or an 8-character backup code. On success, clears the stored secret, enabled timestamp, and backup codes.",
    request_body = TotpDisableRequest,
    responses(
        (status = 204, description = "TOTP was disabled and all related secrets were cleared"),
        (status = 400, description = "Malformed JSON payload, validation error, TOTP is not enabled, or the supplied code is invalid", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_disable(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<TotpDisableRequest>,
) -> Result<StatusCode, Errors> {
    service_totp_disable(&state.db, &state.worker, session.user_id, &payload.code).await?;
    Ok(StatusCode::NO_CONTENT)
}
