use crate::repository::notification::repository_find_notification_preferences_by_user_id;
use crate::service::auth::session_types::SessionContext;
use dto::notification::NotificationPreferenceResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// Fetches the current user's per-channel notification preferences.
///
/// # Role
/// Returns defaults (email/push disabled) if the user has no saved preferences.
///
/// # Related
/// - `repository_find_notification_preferences_by_user_id`
///
/// # Errors
/// - Returns a DB/repository error if the lookup fails.
pub async fn service_get_notification_preferences(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<NotificationPreferenceResponse> {
    let preferences =
        repository_find_notification_preferences_by_user_id(db, session.user_id).await?;

    // If no preferences exist, return default values
    let response = match preferences {
        Some(prefs) => NotificationPreferenceResponse {
            email_enabled: prefs.email_enabled,
            push_enabled: prefs.push_enabled,
            updated_at: prefs.updated_at,
        },
        None => NotificationPreferenceResponse {
            email_enabled: false, // Default: email disabled
            push_enabled: false,  // Default: push disabled
            updated_at: chrono::Utc::now(),
        },
    };

    Ok(response)
}
