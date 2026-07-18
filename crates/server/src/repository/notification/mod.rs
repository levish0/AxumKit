//! Notification repository layer.
//!
//! Provides data access for user notifications and notification preferences
//! (global preferences + per-action preferences).

pub mod action_preferences;
pub mod notification_deliveries;
pub mod preferences;

pub use action_preferences::*;
pub use notification_deliveries::*;
pub use preferences::*;
