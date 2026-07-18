use crate::extractors::RequiredSession;
use crate::service::notification::messages::mark_all_as_read::service_mark_all_notifications_as_read;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/notifications/mark-all-as-read",
    summary = "Mark all notifications as read",
    description = "Marks all notifications for the current authenticated user as read.",
    responses(
        (status = 204, description = "All notifications marked as read"),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn mark_all_notifications_as_read(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<StatusCode, Errors> {
    service_mark_all_notifications_as_read(&state.db, &session_context).await?;
    Ok(StatusCode::NO_CONTENT)
}
