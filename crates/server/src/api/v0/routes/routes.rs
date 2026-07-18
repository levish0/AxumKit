use super::action_logs::routes::action_logs_routes as ActionLogsRoutes;
use super::auth::routes::auth_routes as AuthRoutes;
use super::board::routes::board_routes as BoardRoutes;
use super::groups::routes::group_routes as GroupRoutes;
use super::moderation::routes::moderation_routes as ModerationRoutes;
use super::notification::routes::notification_routes as NotificationRoutes;
use super::search::routes::search_routes as SearchRoutes;
use super::stream::routes::stream_routes as StreamRoutes;
use super::user::routes::user_routes as UserRoutes;
use crate::state::AppState;
use axum::Router;

/// v0 API router
pub fn v0_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(UserRoutes())
        .merge(AuthRoutes(state.clone()))
        .merge(SearchRoutes())
        .merge(ActionLogsRoutes())
        .merge(ModerationRoutes(state.clone()))
        .merge(StreamRoutes())
        .merge(GroupRoutes())
        .merge(NotificationRoutes())
        .merge(BoardRoutes())
}
