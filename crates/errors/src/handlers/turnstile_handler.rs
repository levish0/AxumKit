use crate::errors::Errors;
use crate::protocol::turnstile::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

/// Turnstile error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // Client errors - debug! level
        Errors::TurnstileTokenMissing => {
            debug!("Client error: missing turnstile token");
        }
        Errors::TurnstileVerificationFailed => {
            debug!("Client error: turnstile verification failed");
        }
        // Service errors - warn! level
        Errors::TurnstileServiceError => {
            warn!("Turnstile service error: failed to call Cloudflare API");
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::TurnstileTokenMissing => {
            Some((StatusCode::BAD_REQUEST, TURNSTILE_TOKEN_MISSING, None))
        }
        Errors::TurnstileVerificationFailed => {
            Some((StatusCode::FORBIDDEN, TURNSTILE_VERIFICATION_FAILED, None))
        }
        Errors::TurnstileServiceError => Some((
            StatusCode::SERVICE_UNAVAILABLE,
            TURNSTILE_SERVICE_ERROR,
            None,
        )),

        _ => None,
    }
}
