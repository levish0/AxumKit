use axum::http::{Method, header::HeaderName};
use config::ServerConfig;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tracing::{info, warn};

pub fn cors_layer() -> CorsLayer {
    let config = ServerConfig::get();
    let allowed_origins = if config.cors_allowed_origins.is_empty() {
        if config.is_dev {
            warn!("CORS allowed origins not set, mirroring request origin (dev mode)");
            AllowOrigin::mirror_request()
        } else {
            panic!("CORS_ALLOWED_ORIGINS must be configured in production.");
        }
    } else {
        info!(origins = ?config.cors_allowed_origins, "CORS allowed origins configured");
        AllowOrigin::list(config.cors_allowed_origins.clone())
    };

    let allowed_headers = if config.cors_allowed_headers.is_empty() {
        // Reflect the request's `Access-Control-Request-Headers` rather than using
        // `AllowHeaders::any()`: a wildcard `Access-Control-Allow-Headers: *` is
        // rejected by tower-http when combined with `allow_credentials(true)` (it
        // panics at layer-build time), so `any()` would crash the server on startup
        // whenever CORS_ALLOWED_HEADERS is unset. Mirroring is the credential-safe
        // equivalent of "allow all requested headers".
        warn!("CORS allowed headers not set, mirroring request headers");
        AllowHeaders::mirror_request()
    } else {
        info!(headers = ?config.cors_allowed_headers, "CORS allowed headers configured");
        AllowHeaders::list(config.cors_allowed_headers.clone())
    };

    let max_age = config.cors_max_age.unwrap_or(300);
    info!(max_age_seconds = max_age, "CORS max age configured");

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
        .expose_headers([HeaderName::from_static("x-request-id")])
        .max_age(std::time::Duration::from_secs(max_age))
}
