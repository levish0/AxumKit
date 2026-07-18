use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_status;
use crate::state::AppState;
use axum::extract::State;
use dto::auth::response::TotpStatusResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/auth/totp/status",
    summary = "Get TOTP enrollment status",
    description = "Returns whether TOTP is enabled for the current account, when it was enabled, and how many backup codes remain.",
    responses(
        (status = 200, description = "TOTP enrollment status was returned successfully", body = TotpStatusResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_status(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
) -> Result<TotpStatusResponse, Errors> {
    service_totp_status(&state.db, session.user_id).await
}
