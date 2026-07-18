use chrono::Utc;
use entity::notification_preferences::{
    ActiveModel as NotificationPreferenceActiveModel, Column as NotificationPreferenceColumn,
    Entity as NotificationPreferenceEntity, Model as NotificationPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

/// Partially updates a user's per-channel notification preferences.
///
/// # Role
/// Finds the record for `user_id`, updates only the provided fields, and refreshes `updated_at`.
///
/// # Related
/// - `service_update_notification_preferences`
///
/// # Errors
/// - `Errors::NotFound` if the record does not exist
/// - Returns a DB/repository error if the update fails.
pub async fn repository_update_notification_preferences<C>(
    conn: &C,
    user_id: Uuid,
    email_enabled: Option<bool>,
    push_enabled: Option<bool>,
) -> Result<NotificationPreferenceModel, Errors>
where
    C: ConnectionTrait,
{
    let preferences = NotificationPreferenceEntity::find()
        .filter(NotificationPreferenceColumn::UserId.eq(user_id))
        .one(conn)
        .await?
        .ok_or(Errors::NotFound(
            "Notification preferences not found".to_string(),
        ))?;

    let mut active_prefs: NotificationPreferenceActiveModel = preferences.into();

    if let Some(email_enabled) = email_enabled {
        active_prefs.email_enabled = Set(email_enabled);
    }

    if let Some(push_enabled) = push_enabled {
        active_prefs.push_enabled = Set(push_enabled);
    }

    active_prefs.updated_at = Set(Utc::now());

    let updated = active_prefs.update(conn).await?;

    Ok(updated)
}
