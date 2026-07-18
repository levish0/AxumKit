use entity::notification_deliveries::{
    Column as UserNotificationColumn, Entity as UserNotificationEntity,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};
use uuid::Uuid;

/// Counts a user's unread notifications.
///
/// # Role
/// Returns the number of notifications with `is_read = false` for `user_id`.
///
/// # Callers
/// - `service_count_unread_notifications`
///
/// # Errors
/// - Returns a DB/repository error if the count query fails.
pub async fn repository_count_unread_notifications<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let count = UserNotificationEntity::find()
        .filter(UserNotificationColumn::UserId.eq(user_id))
        .filter(UserNotificationColumn::IsRead.eq(false))
        .count(conn)
        .await?;

    Ok(count)
}
