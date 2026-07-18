use crate::errors::Errors;
use crate::protocol::board::*;
use axum::http::StatusCode;
use tracing::warn;

pub fn log_error(error: &Errors) {
    match error {
        Errors::BoardNotFound | Errors::BoardPostNotFound | Errors::BoardCommentNotFound => {
            warn!(error = ?error, "Board resource not found");
        }
        _ => {}
    }
}

pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::BoardNotFound => Some((StatusCode::NOT_FOUND, BOARD_NOT_FOUND, None)),
        Errors::BoardPostNotFound => Some((StatusCode::NOT_FOUND, BOARD_POST_NOT_FOUND, None)),
        Errors::BoardPostLocked => Some((StatusCode::FORBIDDEN, BOARD_POST_LOCKED, None)),
        // The caller's pin list no longer matches the board's: someone pinned or
        // unpinned since they read it. A conflict, not a bad request — the fix is
        // to re-read and retry, not to change the payload.
        Errors::BoardPinSetMismatch => Some((StatusCode::CONFLICT, BOARD_PIN_SET_MISMATCH, None)),
        Errors::BoardCommentNotFound => {
            Some((StatusCode::NOT_FOUND, BOARD_COMMENT_NOT_FOUND, None))
        }
        _ => None,
    }
}
