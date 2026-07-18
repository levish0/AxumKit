//! User notification repository APIs.
//!
//! Handles notification CRUD, read-state transitions, list queries, and
//! cursor-neighbor existence checks.

mod count_unread;
mod create;
mod delete;
pub mod exists;
mod filter;
mod find_by_user_id_cursor;
mod mark_all_as_read;
mod mark_as_read;

pub use count_unread::repository_count_unread_notifications;
pub use create::repository_create_notification;
pub use delete::{repository_delete_all_notifications_for_user, repository_delete_notification};
pub use exists::*;
pub use filter::NotificationFilter;
pub use find_by_user_id_cursor::{
    NotificationQueryResult, repository_find_notifications_by_user_id_cursor,
};
pub use mark_all_as_read::repository_mark_all_notifications_as_read;
pub use mark_as_read::repository_mark_notification_as_read;
pub use notification_repository::NotificationTarget;
