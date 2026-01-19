use crate::errors::Errors;
use crate::protocol::worker::*;
use axum::http::StatusCode;
use tracing::warn;

/// Worker Service 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // Worker Service 에러 - warn! 레벨 (외부 서비스 관련)
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

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
