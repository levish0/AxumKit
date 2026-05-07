use super::health_check;
use crate::state::AppState;
use axum::{Router, routing::get};

pub fn health_routes() -> Router<AppState> {
    Router::new()
        // Admin status check
        .route("/health-check", get(health_check))
}
