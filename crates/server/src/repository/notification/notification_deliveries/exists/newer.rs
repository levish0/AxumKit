use super::super::filter::{NotificationFilter, apply_notification_filter};
use crate::repository::common::repository_query_exists;
use entity::notification_deliveries::{Column as NotificationColumn, Entity as NotificationEntity};
use errors::errors::Errors;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait,
};
use uuid::Uuid;

/// Checks whether a notification newer than the current cursor exists.
///
/// # Role
/// Checks for the existence of `id > cursor_id` under the `user_id` and filter conditions.
///
/// # Related
/// - `service_get_notifications`
/// - `repository_query_exists`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_exists_newer_notification<C>(
    conn: &C,
    user_id: Uuid,
    filter: &NotificationFilter,
    cursor_id: Uuid,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let query = apply_notification_filter(
        NotificationEntity::find()
            .join(
                JoinType::InnerJoin,
                entity::notification_deliveries::Relation::Event.def(),
            )
            .filter(NotificationColumn::UserId.eq(user_id))
            .filter(NotificationColumn::Id.gt(cursor_id)),
        filter,
    );

    repository_query_exists(conn, query).await
}
