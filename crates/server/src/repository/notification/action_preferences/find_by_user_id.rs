use entity::notification_action_preferences::{
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
    Model as NotificationActionPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Fetches a user's per-action notification preferences.
///
/// # Role
/// Returns all action preference records for the given user.
///
/// # Callers
/// - `service_get_notification_action_preferences`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_find_notification_action_preferences_by_user_id<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Vec<NotificationActionPreferenceModel>, Errors>
where
    C: ConnectionTrait,
{
    let preferences = NotificationActionPreferenceEntity::find()
        .filter(NotificationActionPreferenceColumn::UserId.eq(user_id))
        .all(conn)
        .await?;

    Ok(preferences)
}
