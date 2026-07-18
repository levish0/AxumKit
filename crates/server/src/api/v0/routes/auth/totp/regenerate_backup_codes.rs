use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_regenerate_backup_codes;
use crate::state::AppState;
use axum::extract::State;
use dto::auth::request::TotpRegenerateBackupCodesRequest;
use dto::auth::response::TotpBackupCodesResponse;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/totp/backup-codes/regenerate",
    summary = "Regenerate TOTP backup codes",
    description = "Requires a current TOTP code, replaces the stored backup code set, and returns the new plaintext codes once.",
    request_body = TotpRegenerateBackupCodesRequest,
    responses(
        (status = 200, description = "Backup codes were replaced and returned in plaintext", body = TotpBackupCodesResponse),
        (status = 400, description = "Malformed JSON payload, validation error, TOTP is not enabled, or the supplied TOTP code is invalid", body = ErrorResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth - TOTP"
)]
pub async fn totp_regenerate_backup_codes(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<TotpRegenerateBackupCodesRequest>,
) -> Result<TotpBackupCodesResponse, Errors> {
    service_regenerate_backup_codes(&state.db, session.user_id, &payload.code).await
}
