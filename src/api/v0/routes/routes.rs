use super::user::user::user_routes;
use crate::service::error::errors::handler_404;
use crate::state::AppState;
use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use super::openapi::ApiDoc;

/// API + Swagger UI 라우터 통합
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .merge(SwaggerUi::new("/docs").url("/swagger.json", ApiDoc::openapi()))
        .nest("/v0", user_routes()) // 실제 API 라우터
        .fallback(handler_404)
}

