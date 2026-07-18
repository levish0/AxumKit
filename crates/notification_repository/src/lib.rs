//! Shared notification data access used by both the API server and the worker.
//!
//! Because the server's `repository/` lives inside its binary crate it cannot be
//! shared, so notification queries the worker also needs (who opted out of an
//! action, how an event + deliveries are written) would otherwise live in two
//! places and drift. This crate is the single source for them:
//!
//! - [`deliveries`] — write one event + per-recipient deliveries.
//! - [`preferences`] — filter recipients by their per-action opt-out.
//!
//! Every item is re-exported at the crate root.

pub mod deliveries;
pub mod preferences;

pub use deliveries::{
    NotificationEventInsertSpec, NotificationTarget, insert_notification_event_deliveries,
};
pub use preferences::filter_recipients_by_action_preference;
