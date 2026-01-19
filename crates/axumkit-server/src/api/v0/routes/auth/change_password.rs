use crate::extractors::RequiredSession;
use crate::service::auth::change_password::service_change_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::ChangePasswordRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/change-password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed successfully"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, or incorrect password"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_change_password(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<ChangePasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_change_password(
        &state.conn,
        &state.redis_session,
        session.user_id,
        &session.session_id,
        payload,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
