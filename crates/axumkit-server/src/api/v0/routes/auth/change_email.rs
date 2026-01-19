use crate::extractors::RequiredSession;
use crate::service::auth::change_email::service_change_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::ChangeEmailRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/change-email",
    request_body = ChangeEmailRequest,
    responses(
        (status = 204, description = "Verification email sent to new address"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, or incorrect password"),
        (status = 401, description = "Unauthorized - Invalid or expired session"),
        (status = 500, description = "Internal Server Error")
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_change_email(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<ChangeEmailRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_change_email(
        &state.conn,
        &state.redis_session,
        &state.worker,
        session.user_id,
        payload,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
