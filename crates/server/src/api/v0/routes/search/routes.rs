use crate::state::AppState;
use axum::{Router, routing::get};

use super::search_users::search_users;

pub fn search_routes() -> Router<AppState> {
    // Public routes (no authentication required)
    Router::new().route("/search/users", get(search_users))
}
