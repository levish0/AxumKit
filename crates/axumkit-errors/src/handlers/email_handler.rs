use crate::errors::Errors;
use crate::protocol::email::*;
use axum::http::StatusCode;
use tracing::debug;

/// 이메일 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
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

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
