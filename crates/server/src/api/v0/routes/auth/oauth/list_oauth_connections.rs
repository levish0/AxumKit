use crate::extractors::RequiredSession;
use crate::service::oauth::list_connections::service_list_oauth_connections;
use crate::state::AppState;
use axum::extract::State;
use dto::oauth::response::OAuthConnectionListResponse;
use errors::errors::{ErrorResponse, Errors};

/// Lists the current user's OAuth connections.
#[utoipa::path(
    get,
    path = "/v0/auth/oauth/connections",
    summary = "List OAuth providers linked to the current account",
    description = "Returns every OAuth provider currently linked to the authenticated user so clients can show connected login methods and account settings.",
    responses(
        (status = 200, description = "Linked OAuth providers were returned successfully", body = OAuthConnectionListResponse),
        (status = 401, description = "Missing, invalid, or expired session cookie", body = ErrorResponse),
        (status = 500, description = "Unexpected database error", body = ErrorResponse)
    ),
    tag = "Auth",
    security(
        ("session_id_cookie" = [])
    )
)]
pub async fn list_oauth_connections(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<OAuthConnectionListResponse, Errors> {
    let result = service_list_oauth_connections(&state.db, session_context.user_id).await?;

    Ok(result)
}
