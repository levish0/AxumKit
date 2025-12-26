use crate::dto::auth::internal::anonymous_user::AnonymousUserContext;
use crate::errors::errors::Errors;
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
use redis::AsyncCommands;
use std::net::SocketAddr;
use tracing::{debug, warn};

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

/// Rate limiting middleware by IP address
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
        "Rate limit check: route={}, ip={}, max={}/{}s",
        config.route_name, ip, config.max_requests, config.window_secs
    );

    let mut redis_conn = state.redis_client.clone();

    // Atomically increment the counter
    let count: u32 = redis_conn
        .incr(&key, 1)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis rate limit error: {}", e)))?;

    // Set expiration on first request
    if count == 1 {
        let _: bool = redis_conn
            .expire(&key, config.window_secs as i64)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Redis expire error: {}", e)))?;
    }

    // Check if rate limit exceeded
    if count > config.max_requests {
        // Get TTL for Retry-After header
        let ttl: i64 = redis_conn
            .ttl(&key)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Redis TTL error: {}", e)))?;

        warn!(
            "Rate limit exceeded: route={}, ip={}, count={}/{}, retry_after={}s",
            config.route_name,
            ip,
            count,
            config.max_requests,
            ttl.max(0)
        );

        // Return 429 with rate limit headers
        let mut response = Errors::RateLimitExceeded.into_response();

        let retry_after = ttl.max(0).to_string();
        response
            .headers_mut()
            .insert("Retry-After", HeaderValue::from_str(&retry_after).unwrap());
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&config.max_requests.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", HeaderValue::from_str("0").unwrap());
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            HeaderValue::from_str(&retry_after).unwrap(),
        );

        return Ok(response);
    }

    debug!(
        "Rate limit passed: route={}, ip={}, count={}/{}",
        config.route_name, ip, count, config.max_requests
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add rate limit info headers to successful response
    let remaining = config.max_requests.saturating_sub(count).to_string();
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&config.max_requests.to_string()).unwrap(),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining).unwrap(),
    );

    Ok(response)
}

/// Rate limiting middleware by anonymous session ID
/// Uses anonymous_user_id from AnonymousUserContext for rate limiting
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
        "Rate limit check: route={}, anonymous_id={}, ip={}, max={}/{}s",
        config.route_name, anonymous_id, ip, config.max_requests, config.window_secs
    );

    let mut redis_conn = state.redis_client.clone();

    // Atomically increment the counter
    let count: u32 = redis_conn
        .incr(&key, 1)
        .await
        .map_err(|e| Errors::SysInternalError(format!("Redis rate limit error: {}", e)))?;

    // Set expiration on first request
    if count == 1 {
        let _: bool = redis_conn
            .expire(&key, config.window_secs as i64)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Redis expire error: {}", e)))?;
    }

    // Check if rate limit exceeded
    if count > config.max_requests {
        // Get TTL for Retry-After header
        let ttl: i64 = redis_conn
            .ttl(&key)
            .await
            .map_err(|e| Errors::SysInternalError(format!("Redis TTL error: {}", e)))?;

        warn!(
            "Rate limit exceeded: route={}, anonymous_id={}, ip={}, count={}/{}, retry_after={}s",
            config.route_name,
            anonymous_id,
            ip,
            count,
            config.max_requests,
            ttl.max(0)
        );

        // Return 429 with rate limit headers
        let mut response = Errors::RateLimitExceeded.into_response();

        let retry_after = ttl.max(0).to_string();
        response
            .headers_mut()
            .insert("Retry-After", HeaderValue::from_str(&retry_after).unwrap());
        response.headers_mut().insert(
            "X-RateLimit-Limit",
            HeaderValue::from_str(&config.max_requests.to_string()).unwrap(),
        );
        response
            .headers_mut()
            .insert("X-RateLimit-Remaining", HeaderValue::from_str("0").unwrap());
        response.headers_mut().insert(
            "X-RateLimit-Reset",
            HeaderValue::from_str(&retry_after).unwrap(),
        );

        return Ok(response);
    }

    debug!(
        "Rate limit passed: route={}, anonymous_id={}, ip={}, count={}/{}",
        config.route_name, anonymous_id, ip, count, config.max_requests
    );

    // Process the request
    let mut response = next.run(request).await;

    // Add rate limit info headers to successful response
    let remaining = config.max_requests.saturating_sub(count).to_string();
    response.headers_mut().insert(
        "X-RateLimit-Limit",
        HeaderValue::from_str(&config.max_requests.to_string()).unwrap(),
    );
    response.headers_mut().insert(
        "X-RateLimit-Remaining",
        HeaderValue::from_str(&remaining).unwrap(),
    );

    Ok(response)
}
