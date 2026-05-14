use crate::extractors::RequiredSession;
use crate::service::auth::list_sessions::service_list_sessions;
use crate::state::AppState;
use axum::{extract::State, response::IntoResponse, response::Response};
use dto::auth::response::ListSessionsResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/auth/sessions",
    summary = "List the authenticated user's active sessions",
    description = "Returns every active session bound to the current user, marking the session used by this request with is_current=true. Sessions are sorted by created_at descending.",
    responses(
        (status = 200, description = "Active session list", body = ListSessionsResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected session store error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Auth"
)]
pub async fn auth_list_sessions(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<Response, Errors> {
    let sessions = service_list_sessions(
        &state.redis_session,
        session_context.user_id,
        &session_context.session_id,
    )
    .await?;

    Ok(ListSessionsResponse { sessions }.into_response())
}
