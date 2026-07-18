//! Cursor-neighbor existence checks for user notifications.
//!
//! Used by cursor pagination to calculate `has_newer` and `has_older`.

mod newer;
mod older;

pub use newer::repository_exists_newer_notification;
pub use older::repository_exists_older_notification;
