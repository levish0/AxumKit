use super::auth::routes::auth_routes as AuthRoutes;
use crate::state::AppState;
use axum::Router;

/// v0 API 라우터
pub fn v0_routes(state: AppState) -> Router<AppState> {
    Router::new().merge(AuthRoutes(state.clone()))
}
