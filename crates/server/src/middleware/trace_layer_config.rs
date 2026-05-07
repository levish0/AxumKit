use axum::extract::Request;
use tower_http::request_id::RequestId;
use tracing::Span;

/// Custom MakeSpan implementation that includes request_id in the span
pub fn make_span_with_request_id(request: &Request) -> Span {
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .unwrap_or("unknown");

    tracing::info_span!(
        "request",
        method = %request.method(),
        uri = %request.uri().path(),
        request_id = %request_id,
    )
}
