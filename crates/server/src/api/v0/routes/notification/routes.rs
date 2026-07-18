use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

use super::inbox::count_unread::count_unread_notifications;
use super::inbox::delete_notification::delete_notification;
use super::inbox::get_notifications::get_notifications;
use super::inbox::mark_all_as_read::mark_all_notifications_as_read;
use super::inbox::mark_as_read::mark_notification_as_read;
use super::preferences::get_action_preferences::get_notification_action_preferences;
use super::preferences::get_preferences::get_notification_preferences;
use super::preferences::update_action_preferences_bulk::update_notification_action_preferences_bulk;
use super::preferences::update_preferences::update_notification_preferences;

pub fn notification_routes() -> Router<AppState> {
    // All notification routes require authentication
    Router::new()
        .route("/notifications/list", get(get_notifications))
        .route(
            "/notifications/unread/count",
            get(count_unread_notifications),
        )
        .route(
            "/notifications/mark-as-read",
            post(mark_notification_as_read),
        )
        .route(
            "/notifications/mark-all-as-read",
            post(mark_all_notifications_as_read),
        )
        .route("/notifications/delete", post(delete_notification))
        .route(
            "/notifications/preferences",
            get(get_notification_preferences),
        )
        .route(
            "/notifications/preferences/update",
            post(update_notification_preferences),
        )
        .route(
            "/notifications/preferences/actions",
            get(get_notification_action_preferences),
        )
        .route(
            "/notifications/preferences/actions/update",
            post(update_notification_action_preferences_bulk),
        )
}
