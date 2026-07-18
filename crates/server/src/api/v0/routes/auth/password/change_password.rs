use crate::extractors::RequiredSession;
use crate::service::auth::change_password::service_change_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::ChangePasswordRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/change-password",
    summary = "Change the current account password",
    description = "Re-authenticates the signed-in user with their current password, stores the new password hash, and invalidates every other active session while keeping the current session alive.",
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password was changed and other sessions were invalidated"),
        (status = 400, description = "Malformed JSON payload, validation error, or the new password matches the current password", body = ErrorResponse),
        (status = 401, description = "Missing session, incorrect current password, or this account does not have a password", body = ErrorResponse),
        (status = 500, description = "Unexpected database or session store error", body = ErrorResponse)
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
        &state.db,
        &state.redis_session,
        &state.worker,
        session.user_id,
        &session.session_id,
        payload,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
