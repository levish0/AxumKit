use constants::NotificationAction;
use entity::notification_action_preferences::{
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
    Model as NotificationActionPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Fetches a single notification preference for a user+action combination.
///
/// # Role
/// Looks up whether a record exists for the `user_id + action` condition.
///
/// # Callers
/// - `service_update_notification_action_preference`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_find_notification_action_preference<C>(
    conn: &C,
    user_id: Uuid,
    action: NotificationAction,
) -> Result<Option<NotificationActionPreferenceModel>, Errors>
where
    C: ConnectionTrait,
{
    let preference = NotificationActionPreferenceEntity::find()
        .filter(NotificationActionPreferenceColumn::UserId.eq(user_id))
        .filter(NotificationActionPreferenceColumn::Action.eq(action.to_string()))
        .one(conn)
        .await?;

    Ok(preference)
}
