use axum::http::Method;
use axumkit_config::ServerConfig;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tracing::{info, warn};

pub fn cors_layer() -> CorsLayer {
    let allowed_origins = if ServerConfig::get().cors_allowed_origins.is_empty() {
        warn!("CORS_ALLOWED_ORIGINS is not set, allowing all origins.");
        AllowOrigin::any()
    } else {
        info!(
            "CORS_ALLOWED_ORIGINS is set to {:?}",
            ServerConfig::get().cors_allowed_origins
        );
        AllowOrigin::list(ServerConfig::get().cors_allowed_origins.clone())
    };

    let allowed_headers = if ServerConfig::get().cors_allowed_headers.is_empty() {
        warn!("CORS_ALLOWED_HEADERS is not set, allowing all headers.");
        AllowHeaders::any()
    } else {
        info!(
            "CORS_ALLOWED_HEADERS is set to {:?}",
            ServerConfig::get().cors_allowed_headers
        );
        AllowHeaders::list(ServerConfig::get().cors_allowed_headers.clone())
    };

    let max_age = ServerConfig::get().cors_max_age.unwrap_or(300);
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
        .allow_credentials(true)
        .max_age(std::time::Duration::from_secs(max_age))
}
