use crate::errors::Errors;
use crate::protocol::eventstream::*;
use axum::http::StatusCode;
use tracing::warn;

/// EventStream error logging handler
pub fn log_error(error: &Errors) {
    match error {
        Errors::EventStreamPublishFailed => {
            warn!("EventStream error: {:?}", error);
        }
        _ => {}
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
