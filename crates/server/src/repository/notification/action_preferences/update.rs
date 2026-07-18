use constants::NotificationAction;
use entity::notification_action_preferences::{
    ActiveModel as NotificationActionPreferenceActiveModel,
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
    Model as NotificationActionPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

/// Updates a user's per-action notification preference.
///
/// # Role
/// Finds the `user_id + action` record and updates its `enabled` value.
///
/// # Callers
/// - `service_update_notification_action_preference`
///
/// # Errors
/// - `Errors::NotFound` if the record does not exist
/// - Returns a DB/repository error if the update fails.
pub async fn repository_update_notification_action_preference<C>(
    conn: &C,
    user_id: Uuid,
    action: NotificationAction,
    enabled: bool,
) -> Result<NotificationActionPreferenceModel, Errors>
where
    C: ConnectionTrait,
{
    let preference = NotificationActionPreferenceEntity::find()
        .filter(NotificationActionPreferenceColumn::UserId.eq(user_id))
        .filter(NotificationActionPreferenceColumn::Action.eq(action.to_string()))
        .one(conn)
        .await?
        .ok_or(Errors::NotFound(
            "Notification action preference not found".to_string(),
        ))?;

    let mut active_pref: NotificationActionPreferenceActiveModel = preference.into();
    active_pref.enabled = Set(enabled);

    let updated = active_pref.update(conn).await?;

    Ok(updated)
}
