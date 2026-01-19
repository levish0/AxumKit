use crate::errors::Errors;
use crate::protocol::meilisearch::*;
use axum::http::StatusCode;
use tracing::error;

/// MeiliSearch 에러 로깅 처리
pub fn log_error(err: &Errors) {
    match err {
        Errors::MeiliSearchQueryFailed => {
            error!("MeiliSearch query failed");
        }
        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(err: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match err {
        Errors::MeiliSearchQueryFailed => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            MEILISEARCH_QUERY_FAILED,
            None,
        )),
        _ => None,
    }
}
