use crate::extractors::RequiredSession;
use crate::service::notification::preferences::update_preferences::service_update_notification_preferences;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::{NotificationPreferenceResponse, UpdateNotificationPreferenceRequest};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/notifications/preferences/update",
    summary = "Update notification preferences",
    description = "Updates notification delivery preferences for the current authenticated user.",
    request_body = UpdateNotificationPreferenceRequest,
    responses(
        (status = 200, description = "Notification preferences updated successfully", body = NotificationPreferenceResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn update_notification_preferences(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateNotificationPreferenceRequest>,
) -> Result<NotificationPreferenceResponse, Errors> {
    service_update_notification_preferences(
        &state.db,
        &session_context,
        payload.email_enabled,
        payload.push_enabled,
    )
    .await
}
