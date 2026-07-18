use chrono::Utc;
use entity::notification_deliveries::{
    Column as UserNotificationColumn, Entity as UserNotificationEntity,
};
use errors::errors::Errors;
use sea_orm::prelude::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Marks a single user notification as read.
///
/// # Role
/// Updates `is_read` and `read_at` in a single UPDATE and returns the affected row count.
///
/// # Related
/// - `service_mark_notification_as_read`
///
/// # Errors
/// - Returns a DB/repository error if the update fails.
pub async fn repository_mark_notification_as_read<C>(
    conn: &C,
    notification_id: Uuid,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    // Single atomic UPDATE query without SELECT
    let result = UserNotificationEntity::update_many()
        .col_expr(UserNotificationColumn::IsRead, Expr::value(true))
        .col_expr(
            UserNotificationColumn::ReadAt,
            Expr::value(Some(Utc::now())),
        )
        .filter(UserNotificationColumn::Id.eq(notification_id))
        .filter(UserNotificationColumn::UserId.eq(user_id))
        .filter(UserNotificationColumn::IsRead.eq(false)) // Optimization: skip if already read
        .exec(conn)
        .await?;

    Ok(result.rows_affected)
}
