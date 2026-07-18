use crate::extractors::RequiredSession;
use crate::service::notification::preferences::get_preferences::service_get_notification_preferences;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::NotificationPreferenceResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/notifications/preferences",
    summary = "Get notification preferences",
    description = "Returns the current notification delivery preferences for the authenticated user.",
    responses(
        (status = 200, description = "Notification preferences retrieved successfully", body = NotificationPreferenceResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn get_notification_preferences(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<NotificationPreferenceResponse, Errors> {
    service_get_notification_preferences(&state.db, &session_context).await
}
