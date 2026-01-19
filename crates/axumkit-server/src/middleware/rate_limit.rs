use crate::middleware::anonymous_user::AnonymousUserContext;
use crate::state::AppState;
use crate::utils::extract::extract_ip_address::extract_ip_address;
use axum::http::HeaderMap;
use axum::{
    Extension,
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::HeaderValue,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axumkit_errors::errors::Errors;
use redis::aio::ConnectionManager;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};
use uuid::Uuid;

/// Rate limit configuration
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Route identifier for Redis key (e.g. "auth.login", "auth.oauth_authorize")
    pub route_name: &'static str,
    /// Maximum number of requests allowed in the time window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_secs: u64,
}

/// Sliding Window Rate Limiter script (loaded once, reused)
static SLIDING_WINDOW_SCRIPT: LazyLock<redis::Script> =
    LazyLock::new(|| redis::Script::new(include_str!("lua/sliding_window.lua")));

/// Result from sliding window rate limit check
struct RateLimitResult {
    allowed: bool,
    count: u32,
    retry_after: i64,
}

/// Execute sliding window rate limit check using Lua script
async fn check_sliding_window_rate_limit(
    redis_conn: &mut ConnectionManager,
    key: &str,
    config: &RateLimitConfig,
) -> Result<RateLimitResult, Errors> {
    // Get current timestamp in milliseconds for precision
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Errors::SysInternalError(format!("System time error: {}", e)))?
        .as_millis() as i64;

    // Window in milliseconds
    let window_ms = (config.window_secs * 1000) as i64;

    // Generate unique request ID using UUID v7 (time-ordered, guaranteed unique)
    // This prevents Set collisions when multiple requests arrive in the same millisecond
    let request_id = Uuid::now_v7().to_string();

    // Execute Lua script atomically (script is cached via LazyLock)
    let result: Vec<i64> = SLIDING_WINDOW_SCRIPT
        .key(key)
        .arg(now_ms)
        .arg(window_ms)
        .arg(config.max_requests)
        .arg(&request_id)
        .invoke_async(redis_conn)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis rate limit error: {}", e)))?;

    // Parse result: [allowed (0/1), count, retry_after]
    let allowed = result.first().copied().unwrap_or(0) == 1;
    let count = result.get(1).copied().unwrap_or(0) as u32;
    let retry_after = result.get(2).copied().unwrap_or(0);

    Ok(RateLimitResult {
        allowed,
        count,
        retry_after,
    })
}

/// Rate limiting middleware by IP address
/// Uses sliding window algorithm for accurate rate limiting
/// Kept for future use, not currently active
#[allow(dead_code)]
pub async fn rate_limit_by_ip(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(config): Extension<RateLimitConfig>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    let ip = extract_ip_address(&headers, addr);
    let key = format!("rate_limit:{}:{}", config.route_name, ip);

    debug!(
        "Rate limit check (sliding window): route={}, ip={}, max={}/{}s",
        config.route_name, ip, config.max_requests, config.window_secs
    );

    let mut redis_conn = state.redis_session.clone();

    // Check rate limit using sliding window
    let result = check_sliding_window_rate_limit(&mut redis_conn, &key, &config).await?;

    if !result.allowed {
        warn!(
            "Rate limit exceeded: route={}, ip={}, count={}/{}, retry_after={}s",
            config.route_name, ip, result.count, config.max_requests, result.retry_after
        );

        // Return 429 with rate limit headers
        let mut response = Errors::RateLimitExceeded.into_response();

        let retry_after = result.retry_after.max(0).to_string();
        response.headers_mut().insert(
            "Retry-After",
            HeaderValue::from_str(&retry_after).expect("numeric string is valid header value"),
        );
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&config.max_requests.to_string())
                .expect("numeric string is valid header value"),
        );
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            HeaderValue::from_str(&retry_after).expect("numeric string is valid header value"),
        );

        return Ok(response);
    }

    debug!(
        "Rate limit passed: route={}, ip={}, count={}/{}",
        config.route_name, ip, result.count, config.max_requests
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add rate limit info headers to successful response
    let remaining = config.max_requests.saturating_sub(result.count).to_string();
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&config.max_requests.to_string())
            .expect("numeric string is valid header value"),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining).expect("numeric string is valid header value"),
    );

    Ok(response)
}

/// Rate limiting middleware by anonymous session ID
/// Uses sliding window algorithm for accurate rate limiting
/// IP is kept for logging purposes only
pub async fn rate_limit(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Extension(config): Extension<RateLimitConfig>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Errors> {
    let ip = extract_ip_address(&headers, addr);

    // Get anonymous user ID from extension (set by anonymous_user_middleware)
    let anonymous_id = request
        .extensions()
        .get::<AnonymousUserContext>()
        .map(|ctx| ctx.anonymous_user_id.as_str())
        .unwrap_or("unknown");

    let key = format!("rate_limit:{}:{}", config.route_name, anonymous_id);

    debug!(
        "Rate limit check (sliding window): route={}, anonymous_id={}, ip={}, max={}/{}s",
        config.route_name, anonymous_id, ip, config.max_requests, config.window_secs
    );

    let mut redis_conn = state.redis_session.clone();

    // Check rate limit using sliding window
    let result = check_sliding_window_rate_limit(&mut redis_conn, &key, &config).await?;

    if !result.allowed {
        warn!(
            "Rate limit exceeded: route={}, anonymous_id={}, ip={}, count={}/{}, retry_after={}s",
            config.route_name,
            anonymous_id,
            ip,
            result.count,
            config.max_requests,
            result.retry_after
        );

        // Return 429 with rate limit headers
        let mut response = Errors::RateLimitExceeded.into_response();

        let retry_after = result.retry_after.max(0).to_string();
        response.headers_mut().insert(
            "Retry-After",
            HeaderValue::from_str(&retry_after).expect("numeric string is valid header value"),
        );
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&config.max_requests.to_string())
                .expect("numeric string is valid header value"),
        );
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", HeaderValue::from_static("0"));
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            HeaderValue::from_str(&retry_after).expect("numeric string is valid header value"),
        );

        return Ok(response);
    }

    debug!(
        "Rate limit passed: route={}, anonymous_id={}, ip={}, count={}/{}",
        config.route_name, anonymous_id, ip, result.count, config.max_requests
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add rate limit info headers to successful response
    let remaining = config.max_requests.saturating_sub(result.count).to_string();
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&config.max_requests.to_string())
            .expect("numeric string is valid header value"),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining).expect("numeric string is valid header value"),
    );

    Ok(response)
}
