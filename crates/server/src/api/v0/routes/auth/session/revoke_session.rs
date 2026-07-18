use crate::extractors::RequiredSession;
use crate::service::auth::revoke_session::service_revoke_session;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dto::auth::response::create_logout_response;
use errors::errors::{ErrorResponse, Errors};
use uuid::Uuid;

#[utoipa::path(
    delete,
    path = "/v0/auth/sessions/{management_id}",
    summary = "Revoke one of the authenticated user's active sessions",
    description = "Deletes the specified session from the server. The user must own the session; revoking another user's session returns 404. If the revoked session is the one used by this request, the response also clears the session cookie.",
    params(
        ("management_id" = Uuid, Path, description = "Public session management identifier to revoke")
    ),
    responses(
        (status = 204, description = "The session was revoked"),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 404, description = "Session does not exist or does not belong to the current user", body = ErrorResponse),
        (status = 500, description = "Unexpected session store error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_revoke_session(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    Path(management_id): Path<Uuid>,
) -> Result<Response, Errors> {
    service_revoke_session(&state.redis_session, session_context.user_id, management_id).await?;

    // If the current session was revoked, also clear the cookie to keep client state in sync
    if management_id.to_string() == session_context.management_id {
        return create_logout_response();
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}
