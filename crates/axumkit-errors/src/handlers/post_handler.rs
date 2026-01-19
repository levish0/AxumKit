use crate::errors::Errors;
use crate::protocol::post::*;
use axum::http::StatusCode;
use tracing::warn;

/// Post 관련 에러 로깅 처리
pub fn log_error(error: &Errors) {
    match error {
        Errors::PostNotFound => {
            warn!("Post error: {:?}", error);
        }
        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::PostNotFound => Some((StatusCode::NOT_FOUND, POST_NOT_FOUND, None)),
        _ => None,
    }
}
