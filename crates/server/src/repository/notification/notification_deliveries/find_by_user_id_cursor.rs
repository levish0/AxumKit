use super::filter::{NotificationFilter, apply_notification_filter};
use dto::pagination::CursorDirection;
use entity::common::NotificationType;
use entity::notification_deliveries::{
    Column as NotificationDeliveryColumn, Entity as NotificationDeliveryEntity,
};
use entity::notification_events::Column as NotificationEventColumn;
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};
use uuid::Uuid;

#[derive(Debug, Clone, FromQueryResult)]
pub struct NotificationQueryResult {
    pub id: Uuid,
    pub actor_id: Option<Uuid>,
    pub actor_ip: Option<IpNetwork>,
    pub notification_type: NotificationType,
    pub action: String,
    pub board_id: Option<Uuid>,
    pub post_id: Option<Uuid>,
    pub comment_id: Option<Uuid>,
    pub additional_data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub read_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Fetches a user's notification list using filter/cursor conditions.
pub async fn repository_find_notifications_by_user_id_cursor<C>(
    conn: &C,
    user_id: Uuid,
    cursor_notification_id: Option<Uuid>,
    cursor_direction: Option<CursorDirection>,
    filter: &NotificationFilter,
    limit: u64,
) -> Result<Vec<NotificationQueryResult>, Errors>
where
    C: ConnectionTrait,
{
    let mut query = apply_notification_filter(
        NotificationDeliveryEntity::find()
            .join(
                JoinType::InnerJoin,
                entity::notification_deliveries::Relation::Event.def(),
            )
            .filter(NotificationDeliveryColumn::UserId.eq(user_id))
            .select_only()
            .column_as(NotificationDeliveryColumn::Id, "id")
            .column_as(NotificationEventColumn::ActorId, "actor_id")
            .column_as(NotificationEventColumn::ActorIp, "actor_ip")
            .column_as(
                NotificationEventColumn::NotificationType,
                "notification_type",
            )
            .column_as(NotificationEventColumn::Action, "action")
            .column_as(NotificationEventColumn::BoardId, "board_id")
            .column_as(NotificationEventColumn::PostId, "post_id")
            .column_as(NotificationEventColumn::CommentId, "comment_id")
            .column_as(NotificationEventColumn::AdditionalData, "additional_data")
            .column_as(NotificationDeliveryColumn::IsRead, "is_read")
            .column_as(NotificationDeliveryColumn::CreatedAt, "created_at")
            .column_as(NotificationDeliveryColumn::ReadAt, "read_at"),
        filter,
    );

    if let Some(notification_id) = cursor_notification_id {
        let direction = cursor_direction.unwrap_or(CursorDirection::Older);
        query = match direction {
            CursorDirection::Older => query
                .filter(NotificationDeliveryColumn::Id.lt(notification_id))
                .order_by_desc(NotificationDeliveryColumn::Id),
            CursorDirection::Newer => query
                .filter(NotificationDeliveryColumn::Id.gt(notification_id))
                .order_by_asc(NotificationDeliveryColumn::Id),
        };
    } else {
        query = query.order_by_desc(NotificationDeliveryColumn::Id);
    }

    let notifications = query
        .limit(limit)
        .into_model::<NotificationQueryResult>()
        .all(conn)
        .await?;

    Ok(notifications)
}
