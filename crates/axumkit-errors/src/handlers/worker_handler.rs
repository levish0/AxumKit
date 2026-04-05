use crate::errors::Errors;
use crate::protocol::worker::*;
use axum::http::StatusCode;
use tracing::warn;

/// Worker Service error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // Worker Service errors - warn! level (external service related)
        Errors::WorkerServiceConnectionFailed
        | Errors::WorkerServiceResponseInvalid
        | Errors::VerificationEmailSendFailed
        | Errors::PasswordResetEmailSendFailed => {
            warn!("Worker Service error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::WorkerServiceConnectionFailed => Some((
            StatusCode::SERVICE_UNAVAILABLE,
            WORKER_CONNECTION_FAILED,
            None,
        )),
        Errors::WorkerServiceResponseInvalid => {
            Some((StatusCode::BAD_GATEWAY, WORKER_RESPONSE_INVALID, None))
        }
        Errors::VerificationEmailSendFailed => Some((
            StatusCode::BAD_GATEWAY,
            VERIFICATION_EMAIL_SEND_FAILED,
            None,
        )),
        Errors::PasswordResetEmailSendFailed => Some((
            StatusCode::BAD_GATEWAY,
            PASSWORD_RESET_EMAIL_SEND_FAILED,
            None,
        )),

        _ => None, // Return None for errors from other domains
    }
}
