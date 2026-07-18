use crate::extractors::RequiredSession;
use crate::service::auth::change_email::service_change_email;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ChangeEmailRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/change-email",
    summary = "Start an email change for the current account",
    description = "Re-authenticates the signed-in user with their current password, verifies that the new email is different and unused, stores a one-time email change token, and sends a confirmation email to the new address.",
    request_body = ChangeEmailRequest,
    responses(
        (status = 204, description = "Email change token was created and a confirmation email was queued"),
        (status = 400, description = "Malformed JSON payload, validation error, or the new email matches the current email", body = ErrorResponse),
        (status = 401, description = "Missing session, incorrect current password, or this account does not have a password", body = ErrorResponse),
        (status = 409, description = "The requested email address is already in use", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse),
        (status = 502, description = "Worker service rejected the email change job or returned an invalid response", body = ErrorResponse),
        (status = 503, description = "Worker service could not be reached", body = ErrorResponse)
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
        &state.db,
        &state.redis_session,
        &state.worker,
        session.user_id,
        payload,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
