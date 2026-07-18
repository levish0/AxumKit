use crate::extractors::RequiredSession;
use crate::service::notification::messages::count_unread::service_count_unread_notifications;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::UnreadCountResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/notifications/unread/count",
    summary = "Count unread notifications",
    description = "Returns the number of unread notifications for the current authenticated user.",
    responses(
        (status = 200, description = "Unread count retrieved successfully", body = UnreadCountResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn count_unread_notifications(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<UnreadCountResponse, Errors> {
    service_count_unread_notifications(&state.db, &session_context).await
}
