use crate::service::auth::reset_password::service_reset_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ResetPasswordRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/reset-password",
    summary = "Reset a password with a reset token",
    description = "Consumes a one-time password reset token, stores the new password hash, and invalidates all active sessions for the account.",
    request_body = ResetPasswordRequest,
    responses(
        (status = 204, description = "Password was updated and all active sessions were invalidated"),
        (status = 400, description = "Malformed JSON payload, validation error, or invalid reset token", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_reset_password(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ResetPasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_reset_password(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.token,
        &payload.new_password,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
