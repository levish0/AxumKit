use super::{
    HttpState,
    client_address::client_address,
    error::{ApiError, ErrorDescription, ErrorOutput},
    extract::read_cookie,
};
use axum::{
    Json,
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jobs::TraceContext;
use std::time::{Duration, Instant};
use tracing::Instrument;
use uuid::Uuid;

pub async fn request_context(
    State(state): State<HttpState>,
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = Uuid::now_v7();
    request.extensions_mut().insert(TraceContext { request_id });
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    let timeout_seconds = state.settings.timeout_seconds;
    let span = tracing::info_span!("http_request", request_id=%request_id, method=%request.method(), route, status=tracing::field::Empty, duration_ms=tracing::field::Empty);
    async move {
        let started = Instant::now();
        let validation = validate_origin(&state, request.method(), request.headers());
        let mut response = match validation {
            Err(error) => error.into_response(),
            Ok(()) => match state.request_permits.clone().try_acquire_owned() {
                Err(_) => ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "busy",
                    "The service is busy.",
                )
                .into_response(),
                Ok(_permit) => match tokio::time::timeout(
                    Duration::from_secs(timeout_seconds),
                    run_validated_request(&state, request, next),
                )
                .await
                {
                    Ok(response) => response,
                    Err(_) => ApiError::new(
                        StatusCode::REQUEST_TIMEOUT,
                        "request_timeout",
                        "The request timed out.",
                    )
                    .into_response(),
                },
            },
        };
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let description = response
                .extensions()
                .get::<ErrorDescription>()
                .cloned()
                .unwrap_or_else(|| ErrorDescription {
                    code: match status {
                        StatusCode::NOT_FOUND => "not_found",
                        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
                        StatusCode::PAYLOAD_TOO_LARGE => "body_too_large",
                        _ => "request_failed",
                    },
                    message: status.canonical_reason().unwrap_or("Request failed"),
                });
            *response.body_mut() = Json(ErrorOutput {
                error: description,
                request_id,
            })
            .into_response()
            .into_body();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response.headers_mut().remove(header::CONTENT_LENGTH);
        }
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&request_id.to_string()).expect("UUID header"),
        );
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        response.headers_mut().insert(
            "x-content-type-options",
            HeaderValue::from_static("nosniff"),
        );
        response
            .headers_mut()
            .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
        response.headers_mut().insert(
            "content-security-policy",
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'"),
        );
        if state.settings.secure_cookies {
            response.headers_mut().insert(
                "strict-transport-security",
                HeaderValue::from_static("max-age=31536000"),
            );
        }
        if status == StatusCode::TOO_MANY_REQUESTS
            && !response.headers().contains_key(header::RETRY_AFTER)
        {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, HeaderValue::from_static("60"));
        }
        tracing::Span::current()
            .record("status", status.as_u16())
            .record("duration_ms", started.elapsed().as_millis() as u64);
        tracing::info!("request completed");
        response
    }
    .instrument(span)
    .await
}

async fn run_validated_request(state: &HttpState, request: Request, next: Next) -> Response {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("", MatchedPath::as_str);
    if route.starts_with("/api/v1/auth/") && request.method() != Method::OPTIONS {
        let peer = request
            .extensions()
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|peer| peer.0.ip());
        let client = match client_address(
            peer,
            request.headers(),
            state.settings.internal_proxy_secret.as_ref(),
        ) {
            // In-process requests without a client header or socket peer share one budget.
            Ok(client) => client.map_or_else(|| "in-process".to_owned(), |ip| ip.to_string()),
            Err(error) => return error.into_response(),
        };
        if let Err(error) = state
            .app
            .identity
            .limit_client_authentication(&client)
            .await
        {
            return ApiError::from(error).into_response();
        }
    }
    next.run(request).await
}

fn validate_origin(
    state: &HttpState,
    method: &Method,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    if matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(());
    }
    if headers.contains_key(header::ORIGIN) {
        require_browser_origin(state, headers)?;
    } else if read_cookie(headers, state.settings.session_cookie_name())?.is_some()
        || headers.contains_key("sec-fetch-site")
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "csrf_rejected",
            "A trusted origin is required.",
        ));
    }
    Ok(())
}

pub fn require_browser_origin(state: &HttpState, headers: &HeaderMap) -> Result<(), ApiError> {
    let allowed = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| {
            state
                .settings
                .allowed_origins
                .iter()
                .any(|allowed| allowed == origin)
        });
    if !allowed {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "csrf_rejected",
            "A trusted origin is required.",
        ));
    }
    Ok(())
}
