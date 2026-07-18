use crate::extractors::RequiredSession;
use crate::service::notification::preferences::update_action_preferences_bulk::service_update_action_preferences_bulk;
use crate::state::AppState;
use axum::extract::State;
use dto::notification::{
    NotificationActionPreferenceListResponse, UpdateActionPreferencesBulkRequest,
};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/notifications/preferences/actions/update",
    summary = "Update notification action preferences",
    description = "Replaces per action notification preference overrides for the current authenticated user.",
    request_body = UpdateActionPreferencesBulkRequest,
    responses(
        (status = 200, description = "Action preferences updated successfully", body = NotificationActionPreferenceListResponse),
        (status = 400, description = "Bad request - Invalid JSON or validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or expired session", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "Notifications"
)]
pub async fn update_notification_action_preferences_bulk(
    State(state): State<AppState>,
    RequiredSession(session_context): RequiredSession,
    ValidatedJson(payload): ValidatedJson<UpdateActionPreferencesBulkRequest>,
) -> Result<NotificationActionPreferenceListResponse, Errors> {
    let updates = payload
        .updates
        .into_iter()
        .map(|update| (update.action, update.enabled))
        .collect();

    service_update_action_preferences_bulk(&state.db, &session_context, updates).await
}
