use dto::notification::{
    DeleteNotificationRequest, GetNotificationsRequest, MarkNotificationAsReadRequest,
    NotificationActionPreferenceListResponse, NotificationActionPreferenceResponse,
    NotificationListResponse, NotificationPreferenceResponse, NotificationResponse,
    UnreadCountResponse, UpdateActionPreferenceRequest, UpdateActionPreferencesBulkRequest,
    UpdateNotificationPreferenceRequest,
};
use dto::pagination::CursorDirection;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        super::inbox::get_notifications::get_notifications,
        super::inbox::count_unread::count_unread_notifications,
        super::inbox::mark_as_read::mark_notification_as_read,
        super::inbox::mark_all_as_read::mark_all_notifications_as_read,
        super::inbox::delete_notification::delete_notification,
        super::preferences::get_preferences::get_notification_preferences,
        super::preferences::update_preferences::update_notification_preferences,
        super::preferences::get_action_preferences::get_notification_action_preferences,
        super::preferences::update_action_preferences_bulk::update_notification_action_preferences_bulk,
    ),
    components(
        schemas(
            GetNotificationsRequest,
            CursorDirection,
            NotificationListResponse,
            NotificationResponse,
            UnreadCountResponse,
            MarkNotificationAsReadRequest,
            DeleteNotificationRequest,
            NotificationPreferenceResponse,
            UpdateNotificationPreferenceRequest,
            NotificationActionPreferenceListResponse,
            NotificationActionPreferenceResponse,
            UpdateActionPreferencesBulkRequest,
            UpdateActionPreferenceRequest,
        )
    ),
    tags(
        (name = "Notifications", description = "Notification endpoints")
    )
)]
pub struct NotificationApiDoc;
