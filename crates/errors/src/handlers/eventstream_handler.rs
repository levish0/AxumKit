use crate::errors::Errors;
use crate::protocol::eventstream::*;
use axum::http::StatusCode;
use tracing::warn;

/// EventStream domain error logging.
pub fn log_error(error: &Errors) {
    if let Errors::EventStreamPublishFailed = error {
        warn!(error = ?error, "EventStream error");
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::EventStreamPublishFailed => Some((
            StatusCode::SERVICE_UNAVAILABLE,
            EVENTSTREAM_PUBLISH_FAILED,
            None,
        )),
        _ => None,
    }
}
