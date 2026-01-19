use crate::errors::Errors;
use crate::protocol::turnstile::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

/// Turnstile 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 클라이언트 에러 - debug! 레벨
        Errors::TurnstileTokenMissing => {
            debug!("Client error: missing turnstile token");
        }
        Errors::TurnstileVerificationFailed => {
            debug!("Client error: turnstile verification failed");
        }
        // 서비스 에러 - warn! 레벨
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
