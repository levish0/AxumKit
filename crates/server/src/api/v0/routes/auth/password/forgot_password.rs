use crate::service::auth::forgot_password::service_forgot_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ForgotPasswordRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/forgot-password",
    summary = "Request a password reset email",
    description = "If the submitted email belongs to a password-based account, this endpoint issues a one-time reset token and queues a password reset email. It still returns 204 No Content for unknown emails or OAuth-only accounts to avoid account enumeration.",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 204, description = "Password reset email was queued when the account was eligible"),
        (status = 400, description = "Malformed JSON payload or validation error", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse),
        (status = 502, description = "Worker service rejected the password reset email job or returned an invalid response", body = ErrorResponse),
        (status = 503, description = "Worker service could not be reached", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_forgot_password(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ForgotPasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_forgot_password(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.email,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
