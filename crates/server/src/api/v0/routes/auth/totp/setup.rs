use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_setup;
use crate::state::AppState;
use axum::extract::State;
use dto::auth::response::TotpSetupResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/totp/setup",
    summary = "Start TOTP enrollment",
    description = "Generates a new TOTP secret, stores it as pending on the current user, and returns a QR code plus otpauth URI. TOTP is not enabled until POST /v0/auth/totp/enable verifies the first code.",
    responses(
        (status = 200, description = "Pending TOTP secret and enrollment QR code were created", body = TotpSetupResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 409, description = "TOTP is already enabled for this account", body = ErrorResponse),
        (status = 500, description = "Unexpected secret generation, QR generation, or database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_setup(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
) -> Result<TotpSetupResponse, Errors> {
    service_totp_setup(&state.db, session.user_id).await
}
