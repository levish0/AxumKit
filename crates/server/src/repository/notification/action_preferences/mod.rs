//! Notification action preference repository APIs.
//!
//! Handles create/read/update and bulk upsert for per-action user preferences.

mod create;
mod delete;
mod find_by_user_id;
mod find_by_user_id_and_action;
mod update;
mod upsert_bulk;

pub use create::repository_create_notification_action_preference;
pub use delete::repository_delete_notification_action_preferences_for_user;
pub use find_by_user_id::repository_find_notification_action_preferences_by_user_id;
pub use find_by_user_id_and_action::repository_find_notification_action_preference;
pub use update::repository_update_notification_action_preference;
pub use upsert_bulk::repository_upsert_action_preferences_bulk;
