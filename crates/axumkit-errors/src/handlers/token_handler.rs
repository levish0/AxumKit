use crate::errors::Errors;
use crate::protocol::token::*;
use axum::http::StatusCode;
use tracing::debug;

/// 토큰 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
        Errors::TokenInvalidVerification
        | Errors::TokenExpiredVerification
        | Errors::TokenEmailMismatch
        | Errors::TokenInvalidReset
        | Errors::TokenExpiredReset
        | Errors::TokenInvalidEmailChange => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::TokenInvalidVerification => {
            Some((StatusCode::BAD_REQUEST, TOKEN_INVALID_VERIFICATION, None))
        }
        Errors::TokenExpiredVerification => {
            Some((StatusCode::BAD_REQUEST, TOKEN_EXPIRED_VERIFICATION, None))
        }
        Errors::TokenEmailMismatch => Some((StatusCode::BAD_REQUEST, TOKEN_EMAIL_MISMATCH, None)),
        Errors::TokenInvalidReset => Some((StatusCode::BAD_REQUEST, TOKEN_INVALID_RESET, None)),
        Errors::TokenExpiredReset => Some((StatusCode::BAD_REQUEST, TOKEN_EXPIRED_RESET, None)),
        Errors::TokenInvalidEmailChange => {
            Some((StatusCode::BAD_REQUEST, TOKEN_INVALID_EMAIL_CHANGE, None))
        }

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
