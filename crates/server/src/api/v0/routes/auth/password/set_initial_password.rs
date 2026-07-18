use crate::extractors::RequiredSession;
use crate::service::auth::set_initial_password::service_set_initial_password;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use dto::auth::request::SetInitialPasswordRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/set-initial-password",
    summary = "Set the first account password",
    description = "Stores the first password hash for a signed-in OAuth-only account and invalidates every other active session while keeping the current session alive.",
    request_body = SetInitialPasswordRequest,
    responses(
        (status = 204, description = "Initial password was set and other sessions were invalidated"),
        (status = 400, description = "Malformed JSON payload, validation error, or the account already has a password", body = ErrorResponse),
        (status = 401, description = "Missing session", body = ErrorResponse),
        (status = 500, description = "Unexpected database or session store error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_set_initial_password(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<SetInitialPasswordRequest>,
) -> Result<impl IntoResponse, Errors> {
    service_set_initial_password(
        &state.db,
        &state.redis_session,
        session.user_id,
        &session.session_id,
        &payload.new_password,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
