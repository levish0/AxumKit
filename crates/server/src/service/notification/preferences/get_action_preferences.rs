use crate::repository::notification::repository_find_notification_action_preferences_by_user_id;
use crate::service::auth::session_types::SessionContext;
use constants::NotificationAction;
use dto::notification::{
    NotificationActionPreferenceListResponse, NotificationActionPreferenceResponse,
};
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use std::str::FromStr;

/// Retrieves the currently logged-in user's per-action notification preferences.
///
/// # Role
/// Reads repository results and converts them into a list of `NotificationActionPreferenceResponse`.
///
/// # Related
/// - `repository_find_notification_action_preferences_by_user_id`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn service_get_notification_action_preferences(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<NotificationActionPreferenceListResponse> {
    let preferences =
        repository_find_notification_action_preferences_by_user_id(db, session.user_id).await?;

    let preference_responses: Vec<NotificationActionPreferenceResponse> = preferences
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
