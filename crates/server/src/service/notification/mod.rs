//! Notification service layer.
//!
//! Provides user notification message APIs and per-user notification preference APIs.

pub mod messages;
pub mod notify;
pub mod preferences;

pub use notify::{notify_mentions, service_notify_user};
