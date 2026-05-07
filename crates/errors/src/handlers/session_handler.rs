use crate::errors::Errors;
use crate::protocol::session::*;
use axum::http::StatusCode;
use tracing::debug;

/// Session-related error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // Business logic errors - debug! level (client mistakes)
        Errors::SessionInvalidUserId | Errors::SessionExpired | Errors::SessionNotFound => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::SessionInvalidUserId => {
            Some((StatusCode::UNAUTHORIZED, SESSION_INVALID_USER_ID, None))
        }
        Errors::SessionExpired => Some((StatusCode::UNAUTHORIZED, SESSION_EXPIRED, None)),
        Errors::SessionNotFound => Some((StatusCode::UNAUTHORIZED, SESSION_NOT_FOUND, None)),

        _ => None, // Return None for errors from other domains
    }
}
