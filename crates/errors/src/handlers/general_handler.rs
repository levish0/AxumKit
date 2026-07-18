use crate::errors::Errors;
use crate::protocol::general::*;
use crate::protocol::post::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

/// General domain error logging.
pub fn log_error(error: &Errors) {
    match error {
        // Missing resources
        Errors::PostNotFound => {
            warn!(error = ?error, "Resource not found");
        }

        // Client-side/business validation errors
        Errors::ForbiddenError(_)
        | Errors::BadRequestError(_)
        | Errors::ValidationError(_)
        | Errors::FileTooLargeError(_)
        | Errors::InvalidIpAddress => {
            debug!(error = ?error, "Client error");
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::ForbiddenError(msg) => Some((StatusCode::FORBIDDEN, FORBIDDEN, Some(msg.clone()))),
        Errors::PostNotFound => Some((StatusCode::NOT_FOUND, POST_NOT_FOUND, None)),
        Errors::BadRequestError(msg) => {
            Some((StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg.clone())))
        }
        Errors::ValidationError(msg) => {
            Some((StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg.clone())))
        }
        Errors::FileTooLargeError(msg) => Some((
            StatusCode::PAYLOAD_TOO_LARGE,
            FILE_TOO_LARGE,
            Some(msg.clone()),
        )),
        Errors::InvalidIpAddress => Some((StatusCode::BAD_REQUEST, INVALID_IP_ADDRESS, None)),

        _ => None, // Return None for errors from other domains
    }
}
