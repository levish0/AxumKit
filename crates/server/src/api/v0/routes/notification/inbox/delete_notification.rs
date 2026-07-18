use crate::extractors::RequiredSession;
use crate::service::notification::messages::delete_notification::service_delete_notification;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use dto::notification::DeleteNotificationRequest;
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/notifications/delete",
    summary = "Delete a notification",
    description = "Deletes the requested notification from the current authenticated user notification list.",
    request_body = DeleteNotificationRequest,
    responses(
        (status = 204, description = "Notification deleted successfully"),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 404, description = "Not Found - Notification not found or not owned by user", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn delete_notification(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteNotificationRequest>,
) -> Result<StatusCode, Errors> {
    service_delete_notification(&state.db, &session_context, payload.notification_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
