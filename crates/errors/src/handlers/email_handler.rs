use crate::errors::Errors;
use crate::protocol::email::*;
use axum::http::StatusCode;
use tracing::debug;

/// Email domain error logging.
pub fn log_error(error: &Errors) {
    // Client-side/business validation errors - debug! level
    if let Errors::EmailAlreadyVerified = error {
        debug!(error = ?error, "Client error");
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
