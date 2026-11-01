mod client_address;
mod cookies;
mod error;
mod extract;
mod middleware;
mod validation;

use crate::{bootstrap::Application, config::HttpSettings};
use axum::{
    Router,
    extract::{DefaultBodyLimit, State},
    http::{HeaderValue, Method, StatusCode, header},
    routing::get,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower_http::cors::CorsLayer;

pub use v1::openapi_document;

#[derive(Clone)]
pub struct HttpState {
    pub app: Application,
    pub settings: Arc<HttpSettings>,
    request_permits: Arc<Semaphore>,
}

mod v1;

pub fn build_router(app: Application, settings: HttpSettings) -> Router {
    let origins: Vec<HeaderValue> = settings
        .allowed_origins
        .iter()
        .map(|origin| origin.parse().expect("validated origin"))
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::HEAD,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::RANGE,
            header::IF_RANGE,
        ])
        .expose_headers([
            header::HeaderName::from_static("x-request-id"),
            header::CONTENT_RANGE,
            header::ACCEPT_RANGES,
            header::CONTENT_DISPOSITION,
            header::ETAG,
        ]);
    let state = HttpState {
        app,
        request_permits: Arc::new(Semaphore::new(settings.max_concurrent_requests)),
        settings: Arc::new(settings),
    };
    Router::new()
        .route("/health/live", get(|| async { StatusCode::NO_CONTENT }))
        .route("/health/ready", get(ready))
        .route(
            "/openapi.json",
            get(|| async { axum::Json(openapi_document()) }),
        )
        .nest("/api/v1", v1::routes())
        .layer(DefaultBodyLimit::max(16_384))
        .layer(cors)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::request_context,
        ))
        .layer(sentry::integrations::tower::NewSentryLayer::<
            axum::extract::Request,
        >::new_from_top())
        .with_state(state)
}

async fn ready(State(state): State<HttpState>) -> StatusCode {
    if state.app.is_ready().await {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
