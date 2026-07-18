//! Notification preference service APIs.
//!
//! Covers global notification channel preferences and per-action preferences.

pub mod get_action_preferences;
pub mod get_preferences;
pub mod update_action_preference;
pub mod update_action_preferences_bulk;
pub mod update_preferences;

pub use get_action_preferences::service_get_notification_action_preferences;
pub use get_preferences::service_get_notification_preferences;
pub use update_action_preference::service_update_notification_action_preference;
pub use update_action_preferences_bulk::service_update_action_preferences_bulk;
pub use update_preferences::service_update_notification_preferences;
