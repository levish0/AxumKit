use super::health::routes::health_routes as HealthRoutes;
use super::openapi::ApiDoc;
use crate::errors::errors::handler_404;
use crate::state::AppState;
use axum::Router;
use utoipa_swagger_ui::SwaggerUi;

/// API + Swagger UI 라우터 통합
pub fn api_routes() -> Router<AppState> {
    let mut router = Router::new();

    #[cfg(debug_assertions)]
    {
        router = router.merge(SwaggerUi::new("/docs").url("/swagger.json", ApiDoc::merged()));
    }

    router.nest("/v0", HealthRoutes()).fallback(handler_404)
}
