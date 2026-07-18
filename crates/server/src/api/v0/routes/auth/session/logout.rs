use crate::extractors::RequiredSession;
use crate::service::auth::logout::service_logout;
use crate::state::AppState;
use axum::{extract::State, response::Response};
use dto::auth::response::create_logout_response;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/auth/logout",
    summary = "Invalidate the current session",
    description = "Deletes the server-side session identified by the current session cookie and returns a clearing cookie so the client removes it from the browser.",
    responses(
        (status = 204, description = "The current session was deleted and the cookie was cleared"),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected session store error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_logout(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<Response, Errors> {
    // Handle logout
    service_logout(&state.redis_session, &session_context.session_id).await?;

    // Return a 204 response that clears the cookie
    create_logout_response()
}
