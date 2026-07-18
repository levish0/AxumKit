use constants::NotificationAction;
use entity::notification_action_preferences::{
    ActiveModel as NotificationActionPreferenceActiveModel,
    Model as NotificationActionPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

/// Creates a per-action notification preference record for a user.
///
/// # Role
/// Inserts a new preference record for the `(user_id, action, enabled)` combination.
///
/// # Callers
/// - `service_update_notification_action_preference`
///
/// # Errors
/// - Returns a DB/repository error if the insert fails.
pub async fn repository_create_notification_action_preference<C>(
    conn: &C,
    user_id: Uuid,
    action: NotificationAction,
    enabled: bool,
) -> Result<NotificationActionPreferenceModel, Errors>
where
    C: ConnectionTrait,
{
    let preference = NotificationActionPreferenceActiveModel {
        user_id: Set(user_id),
        action: Set(action.to_string()),
        enabled: Set(enabled),
        ..Default::default()
    };

    let preference = preference.insert(conn).await?;

    Ok(preference)
}
