use crate::errors::Errors;
use crate::protocol::file::*;
use axum::http::StatusCode;
use tracing::warn;

/// File-related error logging handler
pub fn log_error(error: &Errors) {
    match error {
        // File-related errors - warn! level
        Errors::FileUploadError(_)
        | Errors::FileNotFound
        | Errors::FileReadError(_)
        | Errors::FileUnsupportedType(_)
        | Errors::FileProcessingTimeout(_)
        | Errors::FileProcessingUnavailable(_) => {
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
        Errors::FileUnsupportedType(msg) => Some((
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            FILE_UNSUPPORTED_TYPE,
            Some(msg.clone()),
        )),
        Errors::FileProcessingTimeout(msg) => Some((
            StatusCode::GATEWAY_TIMEOUT,
            FILE_PROCESSING_TIMEOUT,
            Some(msg.clone()),
        )),
        Errors::FileProcessingUnavailable(msg) => Some((
            StatusCode::BAD_GATEWAY,
            FILE_PROCESSING_UNAVAILABLE,
            Some(msg.clone()),
        )),

        _ => None, // Return None for errors from other domains
    }
}
