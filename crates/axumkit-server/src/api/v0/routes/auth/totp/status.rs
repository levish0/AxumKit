use crate::extractors::RequiredSession;
use crate::service::auth::totp::service_totp_status;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::auth::response::TotpStatusResponse;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/auth/totp/status",
    responses(
        (status = 200, description = "TOTP status", body = TotpStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
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
    service_totp_status(&state.conn, session.user_id).await
}
