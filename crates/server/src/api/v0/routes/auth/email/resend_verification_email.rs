use crate::service::auth::resend_verification_email::service_resend_verification_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ResendVerificationEmailRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/resend-verification-email",
    summary = "Resend the pending signup verification email",
    description = "Looks up an existing email and password signup that is still pending and resends the same verification token with its remaining validity window. Returns 204 No Content even when no pending signup exists to avoid email enumeration.",
    request_body = ResendVerificationEmailRequest,
    responses(
        (status = 204, description = "Verification email was resent when a pending signup existed"),
        (status = 400, description = "Malformed JSON payload or validation error", body = ErrorResponse),
        (status = 500, description = "Unexpected Redis error", body = ErrorResponse),
        (status = 502, description = "Worker service rejected the verification email job or returned an invalid response", body = ErrorResponse),
        (status = 503, description = "Worker service could not be reached", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_resend_verification_email(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ResendVerificationEmailRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_resend_verification_email(&state.redis_session, &state.worker, &payload.email).await?;

    Ok(StatusCode::NO_CONTENT)
}
