use crate::errors::Errors;
use crate::protocol::email::*;
use axum::http::StatusCode;
use tracing::debug;

/// Email-related error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // Business logic errors - debug! level (client mistakes)
        Errors::EmailAlreadyVerified => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::EmailAlreadyVerified => {
            Some((StatusCode::BAD_REQUEST, EMAIL_ALREADY_VERIFIED, None))
        }

        _ => None, // Return None for errors from other domains
    }
}
