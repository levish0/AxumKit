use crate::config::db_config::DbConfig;
use axum::http::Method;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub fn cors_layer() -> CorsLayer {
    let db_config = DbConfig::from_env();

    let allowed_origins = if db_config.cors_allowed_origins.is_empty() {
        AllowOrigin::any()
    } else {
        AllowOrigin::list(db_config.cors_allowed_origins)
    };

    let allowed_headers = if db_config.cors_allowed_headers.is_empty() {
        vec![]
    } else {
        db_config.cors_allowed_headers
    };

    CorsLayer::new()
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(allowed_headers)
        .allow_origin(allowed_origins)
}
