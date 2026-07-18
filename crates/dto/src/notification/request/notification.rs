use crate::pagination::CursorDirection;
use constants::NotificationAction;
use entity::common::NotificationType;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, ToSchema, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
/// Request payload for get notifications request.
pub struct GetNotificationsRequest {
    /// Cursor notification ID for pagination. None means get latest notifications.
    pub cursor_notification_id: Option<Uuid>,

    /// Pagination direction relative to the cursor. This list reads **newest-first**,
    /// and every page is returned in that order regardless of direction —
    /// `Older`/`Newer` only choose which side of the cursor to read. Defaults to
    /// `Older` (advance toward older items) when a cursor is set; ignored otherwise.
    pub cursor_direction: Option<CursorDirection>,

    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100."))]
    pub limit: u64,

    /// Filter by notification type (board, post, user, system)
    pub notification_type: Option<NotificationType>,

    /// Filter by actions (multiple allowed, OR logic)
    pub actions: Option<Vec<NotificationAction>>,

    /// Filter by read status (true = read only, false = unread only)
    pub is_read: Option<bool>,

    /// Filter by board ID
    pub board_id: Option<Uuid>,

    /// Filter by post ID
    pub post_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
/// Request payload for mark notification as read request.
pub struct MarkNotificationAsReadRequest {
    pub notification_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema, Validate)]
/// Request payload for delete notification request.
pub struct DeleteNotificationRequest {
    pub notification_id: Uuid,
}
