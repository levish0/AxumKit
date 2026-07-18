use crate::service::auth::confirm_email_change::service_confirm_email_change;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ConfirmEmailChangeRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/confirm-email-change",
    summary = "Confirm a pending email change",
    description = "Consumes the email change token that was sent to the new address and updates the stored email if that address is still available.",
    request_body = ConfirmEmailChangeRequest,
    responses(
        (status = 204, description = "Email address was updated successfully"),
        (status = 400, description = "Malformed JSON payload, validation error, or invalid email change token", body = ErrorResponse),
        (status = 409, description = "The new email address is no longer available", body = ErrorResponse),
        (status = 500, description = "Unexpected database or Redis error", body = ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn auth_confirm_email_change(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ConfirmEmailChangeRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_confirm_email_change(
        &state.db,
        &state.redis_session,
        &state.worker,
        &payload.token,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
