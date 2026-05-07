use crate::state::AppState;
use axum::{Router, routing::get};

use super::actions::stream_actions;

pub fn stream_routes() -> Router<AppState> {
    Router::new().route("/eventstream/actions", get(stream_actions))
}
