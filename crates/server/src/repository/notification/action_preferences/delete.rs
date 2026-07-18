use entity::notification_action_preferences::{
    Column as NotificationActionPreferenceColumn, Entity as NotificationActionPreferenceEntity,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes all of a user's per-action notification preferences.
///
/// Removes private settings for data minimization on account deletion.
pub async fn repository_delete_notification_action_preferences_for_user<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = NotificationActionPreferenceEntity::delete_many()
        .filter(NotificationActionPreferenceColumn::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
