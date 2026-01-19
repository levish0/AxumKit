use crate::errors::Errors;
use crate::protocol::file::*;
use axum::http::StatusCode;
use tracing::warn;

/// 파일 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        // 파일 관련 에러 - warn! 레벨
        Errors::FileUploadError(_) | Errors::FileNotFound | Errors::FileReadError(_) => {
            warn!("File/processing error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::FileUploadError(msg) => Some((
            StatusCode::BAD_REQUEST,
            FILE_UPLOAD_ERROR,
            Some(msg.clone()),
        )),
        Errors::FileNotFound => Some((StatusCode::BAD_REQUEST, FILE_NOT_FOUND, None)),
        Errors::FileReadError(msg) => {
            Some((StatusCode::BAD_REQUEST, FILE_READ_ERROR, Some(msg.clone())))
        }

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
