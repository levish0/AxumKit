use super::user::user::user_routes;
use crate::service::error::errors::handler_404;
use crate::state::AppState;
use axum::Router;

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .nest("/v0", user_routes())
        .fallback(handler_404)
}
