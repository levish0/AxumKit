pub(in crate::http::v1) mod contracts;
pub(in crate::http::v1) mod get_notification;
pub(in crate::http::v1) mod get_preferences;
pub(in crate::http::v1) mod list_devices;
pub(in crate::http::v1) mod list_notifications;
pub(in crate::http::v1) mod mark_read;
pub(in crate::http::v1) mod mark_read_batch;
pub(in crate::http::v1) mod register_device;
pub(in crate::http::v1) mod remove_device;
mod routes;
pub(in crate::http::v1) mod save_preferences;
pub(in crate::http::v1) mod unread_count;

pub(super) use routes::routes;
