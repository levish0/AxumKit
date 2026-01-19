use crate::service::auth::reset_password::service_reset_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::ResetPasswordRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    post,
    path = "/v0/auth/reset-password",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "Password reset successfully"),
        (status = 400, description = "Bad request - Invalid JSON, validation error, or invalid/expired token"),
        (status = 500, description = "Internal Server Error")
    ),
    tag = "Auth"
)]
pub async fn auth_reset_password(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ResetPasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_reset_password(
        &state.conn,
        &state.redis_session,
        &payload.token,
        &payload.new_password,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
