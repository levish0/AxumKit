use super::action_logs::routes::action_logs_routes as ActionLogsRoutes;
use super::auth::routes::auth_routes as AuthRoutes;
use super::posts::routes::posts_routes as PostsRoutes;
use super::search::routes::search_routes as SearchRoutes;
use super::stream::routes::stream_routes as StreamRoutes;
use super::user::routes::user_routes as UserRoutes;
use crate::state::AppState;
use axum::Router;

/// v0 API 라우터
pub fn v0_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(UserRoutes())
        .merge(AuthRoutes(state.clone()))
        .merge(PostsRoutes())
        .merge(SearchRoutes())
        .merge(ActionLogsRoutes())
        .merge(StreamRoutes())
}
