use super::{
    get_notification, get_preferences, list_devices, list_notifications, mark_read,
    mark_read_batch, register_device, remove_device, save_preferences, unread_count,
};
use crate::http::HttpState;
use axum::routing::{delete, get, post};

pub(in crate::http::v1) fn routes() -> axum::Router<HttpState> {
    axum::Router::new()
        .route(
            "/notifications",
            get(list_notifications::list_notifications),
        )
        .route(
            "/notifications/unread-count",
            get(unread_count::unread_count),
        )
        .route(
            "/notifications/read-batch",
            post(mark_read_batch::mark_read_batch),
        )
        .route(
            "/notifications/{notification_id}",
            get(get_notification::get_notification),
        )
        .route(
            "/notifications/{notification_id}/read",
            axum::routing::put(mark_read::mark_read),
        )
        .route(
            "/account/notification-preferences",
            get(get_preferences::get_preferences).put(save_preferences::save_preferences),
        )
        .route(
            "/account/push-devices",
            get(list_devices::list_devices).post(register_device::register_device),
        )
        .route(
            "/account/push-devices/{device_id}",
            delete(remove_device::remove_device),
        )
}
