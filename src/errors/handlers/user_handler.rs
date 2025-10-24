use crate::errors::errors::Errors;
use crate::errors::protocol::user::*;
use axum::http::StatusCode;
use tracing::{debug, warn};

pub fn log_error(error: &Errors) {
    match error {
        Errors::UserNotFound => {
            warn!("Resource not found: {:?}", error);
        }

        Errors::UserInvalidPassword
        | Errors::UserPasswordNotSet
        | Errors::UserInvalidSession
        | Errors::UserNotVerified
        | Errors::UserUnauthorized
        | Errors::UserBanned
        | Errors::UserPermissionInsufficient
        | Errors::UserHandleAlreadyExists
        | Errors::UserEmailAlreadyExists
        | Errors::UserTokenExpired
        | Errors::UserNoRefreshToken
        | Errors::UserInvalidToken => {
            debug!("Client error: {:?}", error);
        }

        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        Errors::UserInvalidPassword => {
            Some((StatusCode::UNAUTHORIZED, USER_INVALID_PASSWORD, None))
        }
        Errors::UserPasswordNotSet => Some((StatusCode::UNAUTHORIZED, USER_PASSWORD_NOT_SET, None)),
        Errors::UserInvalidSession => Some((StatusCode::UNAUTHORIZED, USER_INVALID_SESSION, None)),
        Errors::UserNotVerified => Some((StatusCode::UNAUTHORIZED, USER_NOT_VERIFIED, None)),
        Errors::UserNotFound => Some((StatusCode::NOT_FOUND, USER_NOT_FOUND, None)),
        Errors::UserUnauthorized => Some((StatusCode::UNAUTHORIZED, USER_UNAUTHORIZED, None)),
        Errors::UserBanned => Some((StatusCode::FORBIDDEN, USER_BANNED, None)),
        Errors::UserPermissionInsufficient => {
            Some((StatusCode::FORBIDDEN, USER_PERMISSION_INSUFFICIENT, None))
        }
        Errors::UserHandleAlreadyExists => {
            Some((StatusCode::CONFLICT, USER_HANDLE_ALREADY_EXISTS, None))
        }
        Errors::UserEmailAlreadyExists => {
            Some((StatusCode::CONFLICT, USER_EMAIL_ALREADY_EXISTS, None))
        }
        Errors::UserTokenExpired => Some((StatusCode::UNAUTHORIZED, USER_TOKEN_EXPIRED, None)),
        Errors::UserNoRefreshToken => Some((StatusCode::UNAUTHORIZED, USER_NO_REFRESH_TOKEN, None)),
        Errors::UserInvalidToken => Some((StatusCode::UNAUTHORIZED, USER_INVALID_TOKEN, None)),

        _ => None,
    }
}