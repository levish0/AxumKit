use chrono::Utc;
use entity::notification_preferences::{
    ActiveModel as NotificationPreferenceActiveModel, Model as NotificationPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

/// Creates a per-channel notification preference record for a user.
///
/// # Role
/// Inserts a preference record with the initial email/push settings.
///
/// # Related
/// - `service_update_notification_preferences`
///
/// # Errors
/// - Returns a DB/repository error if the insert fails.
pub async fn repository_create_notification_preferences<C>(
    conn: &C,
    user_id: Uuid,
    email_enabled: bool,
    push_enabled: bool,
) -> Result<NotificationPreferenceModel, Errors>
where
    C: ConnectionTrait,
{
    let preferences = NotificationPreferenceActiveModel {
        user_id: Set(user_id),
        email_enabled: Set(email_enabled),
        push_enabled: Set(push_enabled),
        updated_at: Set(Utc::now()),
        ..Default::default()
    };

    let preferences = preferences.insert(conn).await?;

    Ok(preferences)
}
