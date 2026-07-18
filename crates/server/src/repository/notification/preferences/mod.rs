//! Notification channel preference repository APIs.
//!
//! Handles CRUD for per-user global notification channel preferences.

mod create;
mod delete;
mod find_by_user_id;
mod update;

pub use create::repository_create_notification_preferences;
pub use delete::repository_delete_notification_preferences_for_user;
pub use find_by_user_id::repository_find_notification_preferences_by_user_id;
pub use update::repository_update_notification_preferences;
