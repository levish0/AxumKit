use entity::notification_preferences::{
    Column as NotificationPreferenceColumn, Entity as NotificationPreferenceEntity,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes a user's per-channel notification preferences.
///
/// Removes private settings for data minimization on account deletion.
pub async fn repository_delete_notification_preferences_for_user<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = NotificationPreferenceEntity::delete_many()
        .filter(NotificationPreferenceColumn::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
