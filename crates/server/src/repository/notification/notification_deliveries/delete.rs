use entity::notification_deliveries::{
    Column as UserNotificationColumn, Entity as UserNotificationEntity,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes a single notification owned by the user.
///
/// # Role
/// Deletes the notification matching `notification_id + user_id` and returns the affected row count.
///
/// # Related
/// - `service_delete_notification`
///
/// # Errors
/// - Returns a DB/repository error if the delete fails.
pub async fn repository_delete_notification<C>(
    conn: &C,
    notification_id: Uuid,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = UserNotificationEntity::delete_many()
        .filter(UserNotificationColumn::Id.eq(notification_id))
        .filter(UserNotificationColumn::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}

/// Deletes all notifications (the inbox) for a user.
///
/// Removes the recipient-side notification history for data minimization on account deletion.
pub async fn repository_delete_all_notifications_for_user<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = UserNotificationEntity::delete_many()
        .filter(UserNotificationColumn::UserId.eq(user_id))
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
