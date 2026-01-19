use crate::state::AppState;
use axum::{Router, routing::get};

use super::recent_changes::get_action_logs;

pub fn action_logs_routes() -> Router<AppState> {
    Router::new().route("/action-logs", get(get_action_logs))
}
