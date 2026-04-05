use crate::bridge::worker_client;
use crate::service::auth::verify_email::service_verify_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axumkit_dto::auth::request::VerifyEmailRequest;
use axumkit_dto::validator::json_validator::ValidatedJson;
use axumkit_errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/verify-email",
    summary = "Complete an email signup with a verification token",
    description = "Consumes the pending email verification token, creates the user account if the email and handle are still available, and then schedules background indexing. The token is only cleaned up after the database commit succeeds.",
    request_body = VerifyEmailRequest,
    responses(
        (status = 204, description = "Verification token accepted and the account was created"),
        (status = 400, description = "Malformed JSON payload, validation error, or invalid verification token", body = ErrorResponse),
        (status = 409, description = "The email or handle became unavailable before the account was created", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_verify_email(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<VerifyEmailRequest>,
) -> Result<impl IntoResponse, Errors> {
    let user_id = service_verify_email(&state.db, &state.redis_session, &payload.token).await?;

    worker_client::index_user(&state.worker, user_id).await.ok();

    Ok(StatusCode::NO_CONTENT)
}
