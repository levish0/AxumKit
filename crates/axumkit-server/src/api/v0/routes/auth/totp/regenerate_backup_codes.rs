use crate::extractors::RequiredSession;
use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::totp::service_regenerate_backup_codes;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::auth::request::TotpRegenerateBackupCodesRequest;
use axumkit_dto::auth::response::TotpBackupCodesResponse;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/totp/backup-codes/regenerate",
    request_body = TotpRegenerateBackupCodesRequest,
    responses(
        (status = 200, description = "Backup codes regenerated", body = TotpBackupCodesResponse),
        (status = 400, description = "Invalid TOTP code or TOTP not enabled"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
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
    let user = repository_get_user_by_id(&state.write_db, session.user_id).await?;
    service_regenerate_backup_codes(&state.write_db, session.user_id, &user.email, &payload.code).await
}
