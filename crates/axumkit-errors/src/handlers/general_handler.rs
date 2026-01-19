use crate::errors::Errors;
use crate::protocol::general::*;
use crate::protocol::post::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

/// 일반 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 리소스 찾을 수 없음 - warn! 레벨
        Errors::PostNotFound => {
            warn!("Resource not found: {:?}", error);
        }

        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
        Errors::ForbiddenError(_)
        | Errors::BadRequestError(_)
        | Errors::ValidationError(_)
        | Errors::FileTooLargeError(_)
        | Errors::InvalidIpAddress => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::ForbiddenError(msg) => Some((StatusCode::FORBIDDEN, FORBIDDEN, Some(msg.clone()))),
        Errors::PostNotFound => Some((StatusCode::NOT_FOUND, POST_NOT_FOUND, None)),
        Errors::BadRequestError(msg) => {
            Some((StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg.clone())))
        }
        Errors::ValidationError(msg) => {
            Some((StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg.clone())))
        }
        Errors::FileTooLargeError(msg) => Some((
            StatusCode::PAYLOAD_TOO_LARGE,
            FILE_TOO_LARGE,
            Some(msg.clone()),
        )),
        Errors::InvalidIpAddress => Some((StatusCode::BAD_REQUEST, INVALID_IP_ADDRESS, None)),

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
