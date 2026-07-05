use crate::extractors::RequiredSession;
use crate::service::user::account::delete_my_account::{
    AccountDeletionOutcome, service_request_account_deletion,
};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dto::auth::response::create_logout_response;
use dto::user::DeleteMyAccountRequest;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    delete,
    path = "/v0/user/me",
    summary = "Delete my account",
    description = "Deletes the current account after re-authentication (OWASP ASVS 7.5.1). \
        Password accounts must supply `password`; OAuth-only accounts with TOTP must supply \
        `totp_code`. OAuth-only accounts with no inline factor receive a confirmation email \
        instead and the response is 202 (deletion completes at the confirm endpoint).",
    request_body = DeleteMyAccountRequest,
    responses(
        (status = 204, description = "Account deleted successfully"),
        (status = 202, description = "Confirmation email sent; deletion pending confirmation"),
        (status = 401, description = "Unauthorized - Invalid session or missing/failed re-authentication", body = ErrorResponse),
        (status = 404, description = "Not Found - User not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or session error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "User",
)]
pub async fn delete_my_account(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    payload: Option<Json<DeleteMyAccountRequest>>,
) -> Result<Response, Errors> {
    let payload = payload.map(|Json(body)| body).unwrap_or_default();

    let outcome = service_request_account_deletion(
        &state.db,
        &state.redis_session,
        &state.worker,
        &session_context,
        payload,
    )
    .await?;

    match outcome {
        // Deletion invalidated every session, so clear the cookie and 204.
        AccountDeletionOutcome::Deleted => create_logout_response(),
        // Deferred to email confirmation: the session stays valid until confirmed.
        AccountDeletionOutcome::ConfirmationEmailSent => Ok(StatusCode::ACCEPTED.into_response()),
    }
}
