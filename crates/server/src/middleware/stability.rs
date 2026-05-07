use axum::{BoxError, http::StatusCode};

/// Handle errors from tower stability middleware (timeout, buffer)
pub async fn handle_tower_error(err: BoxError) -> (StatusCode, &'static str) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (StatusCode::REQUEST_TIMEOUT, "Request timed out")
    } else if err.is::<tower::buffer::error::Closed>() {
        // Buffer worker unexpectedly terminated
        (StatusCode::SERVICE_UNAVAILABLE, "Service unavailable")
    } else {
        // ServiceError or other errors
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
    }
}
