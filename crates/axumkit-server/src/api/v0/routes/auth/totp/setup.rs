use crate::extractors::RequiredSession;
use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::totp::service_totp_setup;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::auth::response::TotpSetupResponse;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/totp/setup",
    responses(
        (status = 200, description = "TOTP setup initiated", body = TotpSetupResponse),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "TOTP already enabled"),
        (status = 500, description = "Internal Server Error")
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
    let user = repository_get_user_by_id(&state.write_db, session.user_id).await?;
    service_totp_setup(&state.write_db, session.user_id, &user.email).await
}
