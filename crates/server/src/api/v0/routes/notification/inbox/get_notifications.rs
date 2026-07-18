use crate::extractors::RequiredSession;
use crate::service::notification::messages::get_notifications::service_get_notifications;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::{GetNotificationsRequest, NotificationListResponse};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/notifications/list",
    summary = "List notifications",
    description = "Returns notifications for the current authenticated user, applying the submitted filters and pagination parameters.",
    params(GetNotificationsRequest),
    responses(
        (status = 200, description = "Notifications retrieved successfully", body = NotificationListResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn get_notifications(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedQuery(payload): ValidatedQuery<GetNotificationsRequest>,
) -> Result<NotificationListResponse, Errors> {
    service_get_notifications(&state.db, &session_context, payload).await
}
