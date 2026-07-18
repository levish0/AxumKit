use crate::extractors::RequiredSession;
use crate::service::notification::messages::mark_notification_as_read::service_mark_notification_as_read;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use dto::notification::MarkNotificationAsReadRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/notifications/mark-as-read",
    summary = "Mark a notification as read",
    description = "Marks the requested notification as read for the current authenticated user.",
    request_body = MarkNotificationAsReadRequest,
    responses(
        (status = 204, description = "Notification marked as read"),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 404, description = "Not Found - Notification not found or already read", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn mark_notification_as_read(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<MarkNotificationAsReadRequest>,
) -> Result<StatusCode, Errors> {
    service_mark_notification_as_read(&state.db, &session_context, payload.notification_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
