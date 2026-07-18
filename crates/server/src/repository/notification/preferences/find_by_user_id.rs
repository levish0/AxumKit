use entity::notification_preferences::{
    Column as NotificationPreferenceColumn, Entity as NotificationPreferenceEntity,
    Model as NotificationPreferenceModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Looks up a user's per-channel notification preferences by user ID.
///
/// # Role
/// Returns the single preference record keyed by `user_id`.
///
/// # Related
/// - `service_get_notification_preferences`
/// - `service_update_notification_preferences`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_find_notification_preferences_by_user_id<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<NotificationPreferenceModel>, Errors>
where
    C: ConnectionTrait,
{
    let preferences = NotificationPreferenceEntity::find()
        .filter(NotificationPreferenceColumn::UserId.eq(user_id))
        .one(conn)
        .await?;

    Ok(preferences)
}
