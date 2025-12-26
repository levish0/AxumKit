use super::health::routes::health_routes;
use super::openapi::ApiDoc;
use super::v0::routes::routes::v0_routes;
use crate::errors::errors::handler_404;
use crate::state::AppState;
use axum::Router;
use utoipa_swagger_ui::SwaggerUi;

/// 최상위 API 라우터 (health + versioned APIs)
pub fn api_routes(state: AppState) -> Router<AppState> {
    let mut router = Router::new();

    #[cfg(debug_assertions)]
    {
        router = router.merge(SwaggerUi::new("/docs").url("/swagger.json", ApiDoc::merged()));
    }

    router
        .merge(health_routes())
        .nest("/v0", v0_routes(state))
        .fallback(handler_404)
}
