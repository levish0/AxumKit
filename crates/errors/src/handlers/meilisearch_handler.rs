use crate::errors::Errors;
use crate::protocol::meilisearch::*;
use axum::http::StatusCode;
use tracing::error;

/// MeiliSearch domain error logging.
pub fn log_error(err: &Errors) {
    if let Errors::MeiliSearchQueryFailed = err {
        error!("MeiliSearch query failed");
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
        Errors::ReindexAlreadyRunning => {
            Some((StatusCode::CONFLICT, REINDEX_ALREADY_RUNNING, None))
        }
        _ => None,
    }
}
