use crate::repository::notification::repository_upsert_action_preferences_bulk;
use crate::service::auth::session_types::SessionContext;
use constants::NotificationAction;
use dto::notification::{
    NotificationActionPreferenceListResponse, NotificationActionPreferenceResponse,
};
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use std::str::FromStr;

/// Bulk-upserts the current user's per-action notification preferences.
///
/// # Role
/// Bulk-upserts the `(action, enabled)` list via the repository and maps it to the response DTO.
///
/// # Related
/// - `repository_upsert_action_preferences_bulk`
///
/// # Errors
/// - Returns a DB/repository error if the save fails.
pub async fn service_update_action_preferences_bulk(
    db: &DatabaseConnection,
    session: &SessionContext,
    updates: Vec<(NotificationAction, bool)>,
) -> ServiceResult<NotificationActionPreferenceListResponse> {
    // No transaction needed - INSERT ON CONFLICT is already atomic
    let results = repository_upsert_action_preferences_bulk(db, session.user_id, updates).await?;

    let preference_responses: Vec<NotificationActionPreferenceResponse> = results
        .into_iter()
        .filter_map(|pref| {
            NotificationAction::from_str(&pref.action)
                .ok()
                .map(|action| NotificationActionPreferenceResponse {
                    action,
                    enabled: pref.enabled,
                })
        })
        .collect();

    Ok(NotificationActionPreferenceListResponse {
        preferences: preference_responses,
    })
}
