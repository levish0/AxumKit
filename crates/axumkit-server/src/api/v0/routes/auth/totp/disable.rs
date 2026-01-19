use crate::extractors::RequiredSession;
use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::totp::service_totp_disable;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axumkit_dto::auth::request::TotpDisableRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/totp/disable",
    request_body = TotpDisableRequest,
    responses(
        (status = 204, description = "TOTP disabled"),
        (status = 400, description = "Invalid TOTP code or TOTP not enabled"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal Server Error")
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
    let user = repository_get_user_by_id(&state.conn, session.user_id).await?;
    service_totp_disable(&state.conn, session.user_id, &user.email, &payload.code).await?;
    Ok(StatusCode::NO_CONTENT)
}
