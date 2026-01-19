use crate::errors::Errors;
use crate::protocol::password::*;
use axum::http::StatusCode;
use tracing::debug;

pub fn log_error(error: &Errors) {
    match error {
        Errors::PasswordRequiredForUpdate
        | Errors::PasswordIncorrect
        | Errors::PasswordCannotUpdateOauthOnly
        | Errors::PasswordNewPasswordMissing
        | Errors::PasswordAlreadySet => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::PasswordRequiredForUpdate => {
            Some((StatusCode::BAD_REQUEST, PASSWORD_REQUIRED_FOR_UPDATE, None))
        }
        Errors::PasswordIncorrect => Some((StatusCode::BAD_REQUEST, PASSWORD_INCORRECT, None)),
        Errors::PasswordCannotUpdateOauthOnly => Some((
            StatusCode::BAD_REQUEST,
            PASSWORD_CANNOT_UPDATE_OAUTH_ONLY,
            None,
        )),
        Errors::PasswordNewPasswordMissing => {
            Some((StatusCode::BAD_REQUEST, PASSWORD_NEW_PASSWORD_MISSING, None))
        }
        Errors::PasswordAlreadySet => Some((StatusCode::BAD_REQUEST, PASSWORD_ALREADY_SET, None)),

        _ => None, // 다른 도메인의 에러는 None 반환
    }
}
