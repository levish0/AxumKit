//! Notification message service APIs.
//!
//! Covers listing, read-state updates, deletion, and unread count.

pub mod count_unread;
pub mod delete_notification;
pub mod get_notifications;
pub mod mark_all_as_read;
pub mod mark_notification_as_read;

pub use count_unread::service_count_unread_notifications;
pub use delete_notification::service_delete_notification;
pub use get_notifications::service_get_notifications;
pub use mark_all_as_read::service_mark_all_notifications_as_read;
pub use mark_notification_as_read::service_mark_notification_as_read;
