use crate::errors::Errors;
use crate::protocol::session::*;
use axum::http::StatusCode;
use tracing::debug;

/// 세션 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
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

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
