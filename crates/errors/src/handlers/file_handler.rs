use crate::errors::Errors;
use crate::protocol::file::*;
use axum::http::StatusCode;
use tracing::warn;

/// File domain error logging.
pub fn log_error(error: &Errors) {
    match error {
        // File errors - warn! level
        Errors::FileUploadError(_)
        | Errors::FileNotFound
        | Errors::FileReadError(_)
        | Errors::FileUnsupportedType(_)
        | Errors::FileProcessingTimeout(_)
        | Errors::FileProcessingUnavailable(_) => {
            warn!(error = ?error, "File/processing error");
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
        Errors::FileNotFound => Some((StatusCode::NOT_FOUND, FILE_NOT_FOUND, None)),
        Errors::FileReadError(msg) => Some((
            StatusCode::INTERNAL_SERVER_ERROR,
            FILE_READ_ERROR,
            Some(msg.clone()),
        )),
        Errors::FileUnsupportedType(msg) => Some((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            FILE_UNSUPPORTED_TYPE,
            Some(msg.clone()),
        )),
        Errors::FileProcessingTimeout(msg) => Some((
            StatusCode::REQUEST_TIMEOUT,
            FILE_PROCESSING_TIMEOUT,
            Some(msg.clone()),
        )),
        Errors::FileProcessingUnavailable(msg) => Some((
            StatusCode::SERVICE_UNAVAILABLE,
            FILE_PROCESSING_UNAVAILABLE,
            Some(msg.clone()),
        )),

        _ => None, // Return None for errors from other domains
    }
}
