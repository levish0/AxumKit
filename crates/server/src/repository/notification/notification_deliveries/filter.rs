use constants::NotificationAction;
use entity::common::NotificationType;
use entity::notification_deliveries::{
    Column as NotificationDeliveryColumn, Entity as NotificationDeliveryEntity,
};
use entity::notification_events::Column as NotificationEventColumn;
use sea_orm::{ColumnTrait, QueryFilter, Select};
use uuid::Uuid;

/// Filter options for notification queries
#[derive(Debug, Default, Clone)]
pub struct NotificationFilter {
    pub notification_type: Option<NotificationType>,
    pub actions: Option<Vec<NotificationAction>>,
    pub is_read: Option<bool>,
    pub board_id: Option<Uuid>,
    pub post_id: Option<Uuid>,
}

pub(crate) fn apply_notification_filter(
    mut query: Select<NotificationDeliveryEntity>,
    filter: &NotificationFilter,
) -> Select<NotificationDeliveryEntity> {
    if let Some(notification_type) = &filter.notification_type {
        query =
            query.filter(NotificationEventColumn::NotificationType.eq(notification_type.clone()));
    }

    if let Some(actions) = &filter.actions
        && !actions.is_empty()
    {
        let action_strings: Vec<String> = actions.iter().map(|a| a.to_string()).collect();
        query = query.filter(NotificationEventColumn::Action.is_in(action_strings));
    }

    if let Some(is_read) = filter.is_read {
        query = query.filter(NotificationDeliveryColumn::IsRead.eq(is_read));
    }

    if let Some(board_id) = filter.board_id {
        query = query.filter(NotificationEventColumn::BoardId.eq(board_id));
    }

    if let Some(post_id) = filter.post_id {
        query = query.filter(NotificationEventColumn::PostId.eq(post_id));
    }

    query
}
