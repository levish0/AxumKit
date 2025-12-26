use crate::errors::errors::Errors;
use crate::errors::protocol::general::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

/// 일반 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 비즈니스 로직 에러 - debug! 레벨 (클라이언트 실수)
        Errors::ForbiddenError(_)
        | Errors::BadRequestError(_)
        | Errors::ValidationError(_)
        | Errors::FileTooLargeError(_) => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::ForbiddenError(msg) => {
            Some((StatusCode::FORBIDDEN, "FORBIDDEN", Some(msg.clone())))
        }
        Errors::BadRequestError(msg) => {
            Some((StatusCode::BAD_REQUEST, BAD_REQUEST, Some(msg.clone())))
        }
        Errors::ValidationError(msg) => {
            Some((StatusCode::BAD_REQUEST, VALIDATION_ERROR, Some(msg.clone())))
        }
        Errors::FileTooLargeError(msg) => Some((
            StatusCode::PAYLOAD_TOO_LARGE,
            "FILE_TOO_LARGE",
            Some(msg.clone()),
        )),

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
