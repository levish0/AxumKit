use crate::errors::Errors;
use crate::protocol::system::*;
use axum::http::StatusCode;
use tracing::{error, warn};

/// System domain error logging.
pub fn log_error(err: &Errors) {
    match err {
        // System-severity errors - error! level
        Errors::SysInternalError(_)
        | Errors::DatabaseError(_)
        | Errors::TransactionError(_)
        | Errors::HashingError(_)
        | Errors::TokenCreationError(_) => {
            error!(error = ?err, "System error occurred");
        }

        // Missing resources - warn! level
        Errors::NotFound(_) => {
            warn!(error = ?err, "Resource not found");
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::SysInternalError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            SYS_INTERNAL_ERROR,
            Some(msg.clone()),
        )),
        Errors::TransactionError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            SYS_TRANSACTION_ERROR,
            Some(msg.clone()),
        )),
        Errors::DatabaseError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            SYS_DATABASE_ERROR,
            Some(msg.clone()),
        )),
        Errors::NotFound(msg) => Some((StatusCode::NOT_FOUND, SYS_NOT_FOUND, Some(msg.clone()))),
        Errors::HashingError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            SYS_HASHING_ERROR,
            Some(msg.clone()),
        )),
        Errors::TokenCreationError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            SYS_TOKEN_CREATION_ERROR,
            Some(msg.clone()),
        )),

        _ => None, // Return None for errors from other domains
    }
}
