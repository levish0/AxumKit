use crate::errors::Errors;
use crate::protocol::rate_limit::*;
use axum::http::StatusCode;
use tracing::warn;

/// Rate-limit domain error logging.
pub fn log_error(error: &Errors) {
    if let Errors::RateLimitExceeded = error {
        warn!("Rate limit exceeded");
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::RateLimitExceeded => {
            Some((StatusCode::TOO_MANY_REQUESTS, RATE_LIMIT_EXCEEDED, None))
        }
        _ => None,
    }
}
