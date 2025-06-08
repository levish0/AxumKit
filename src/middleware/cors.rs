use crate::config::db_config::DbConfig;
use axum::http::Method;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tracing::{info, warn};

pub fn cors_layer() -> CorsLayer {
    let allowed_origins = if DbConfig::get().cors_allowed_origins.is_empty() {
        warn!("CORS_ALLOWED_ORIGINS is not set, allowing all origins.");
        AllowOrigin::any()
    } else {
        info!(
            "CORS_ALLOWED_ORIGINS is set to {:?}",
            DbConfig::get().cors_allowed_origins
        );
        AllowOrigin::list(DbConfig::get().cors_allowed_origins.clone())
    };

    let allowed_headers = if DbConfig::get().cors_allowed_headers.is_empty() {
        warn!("CORS_ALLOWED_HEADERS is not set, allowing all headers.");
        AllowHeaders::any()
    } else {
        info!(
            "CORS_ALLOWED_HEADERS is set to {:?}",
            DbConfig::get().cors_allowed_headers
        );
        AllowHeaders::list(DbConfig::get().cors_allowed_headers.clone())
    };

    let max_age = DbConfig::get().cors_max_age.unwrap_or(300);
    info!("Setting CORS max_age to {} seconds", max_age);

    CorsLayer::new()
        .allow_methods(vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_headers(allowed_headers)
        .allow_origin(allowed_origins)
        .max_age(std::time::Duration::from_secs(max_age))
}
