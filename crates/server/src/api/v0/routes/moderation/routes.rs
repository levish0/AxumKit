use crate::state::AppState;
use axum::{Router, routing::get};

use super::list_logs::list_moderation_logs;

pub fn moderation_routes() -> Router<AppState> {
    Router::new().route("/moderation/logs", get(list_moderation_logs))
}
