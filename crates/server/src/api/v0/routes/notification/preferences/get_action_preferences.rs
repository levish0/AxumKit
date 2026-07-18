use crate::extractors::RequiredSession;
use crate::service::notification::preferences::get_action_preferences::service_get_notification_action_preferences;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::NotificationActionPreferenceListResponse;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/notifications/preferences/actions",
    summary = "Get notification action preferences",
    description = "Returns per action notification preference overrides for the current authenticated user.",
    responses(
        (status = 200, description = "Action preferences retrieved successfully", body = NotificationActionPreferenceListResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn get_notification_action_preferences(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
) -> Result<NotificationActionPreferenceListResponse, Errors> {
    service_get_notification_action_preferences(&state.db, &session_context).await
}
