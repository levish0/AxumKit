use crate::middleware::require_role::require_mod;
use crate::state::AppState;
use axum::middleware::from_fn_with_state;
use axum::{Router, routing::get};

use super::list_logs::list_moderation_logs;

pub fn moderation_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/moderation/logs", get(list_moderation_logs))
        // Router-boundary gate: every moderation route requires at least the Mod role. Keeping the
        // check here (not in each handler) makes "moderation is privileged" a single, greppable
        // property of the route table.
        .route_layer(from_fn_with_state(state, require_mod))
}
