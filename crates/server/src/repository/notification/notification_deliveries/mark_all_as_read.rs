use chrono::Utc;
use entity::notification_deliveries::{
    Column as UserNotificationColumn, Entity as UserNotificationEntity,
};
use errors::errors::Errors;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, UpdateResult, sea_query::Expr,
};
use uuid::Uuid;

/// Marks all of a user's unread notifications as read.
///
/// # Role
/// Bulk-updates records with `is_read = false` and returns the `UpdateResult`.
///
/// # Related
/// - `service_mark_all_notifications_as_read`
///
/// # Errors
/// - Returns a DB/repository error if the update fails.
pub async fn repository_mark_all_notifications_as_read<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<UpdateResult, Errors>
where
    C: ConnectionTrait,
{
    let result = UserNotificationEntity::update_many()
        .col_expr(UserNotificationColumn::IsRead, Expr::value(true))
        .col_expr(
            UserNotificationColumn::ReadAt,
            Expr::value(Some(Utc::now())),
        )
        .filter(UserNotificationColumn::UserId.eq(user_id))
        .filter(UserNotificationColumn::IsRead.eq(false))
        .exec(conn)
        .await?;

    Ok(result)
}
