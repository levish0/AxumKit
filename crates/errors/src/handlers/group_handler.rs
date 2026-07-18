use crate::errors::Errors;
use crate::protocol::group::*;
use crate::protocol::permission::*;
use axum::http::StatusCode;
use tracing::debug;

/// ACL domain error logging.
pub fn log_error(error: &Errors) {
    match error {
        Errors::PermissionDenied(_)
        | Errors::GroupNotFound
        | Errors::GroupAlreadyExists
        | Errors::GroupIsSystem
        | Errors::GroupMemberNotFound
        | Errors::GroupMemberAlreadyExists
        | Errors::InvalidPermission(_) => {
            debug!(error = ?error, "Client error");
        }
        _ => {}
    }
}

/// Returns: (StatusCode, error_code, details)
pub fn map_response(error: &Errors) -> Option<(StatusCode, &'static str, Option<String>)> {
    match error {
        // The detail names the matched rule/condition so users can see
        // exactly why they were denied.
        Errors::PermissionDenied(detail) => Some((
            StatusCode::FORBIDDEN,
            PERMISSION_DENIED,
            Some(detail.clone()),
        )),
        Errors::GroupNotFound => Some((StatusCode::NOT_FOUND, GROUP_NOT_FOUND, None)),
        Errors::GroupAlreadyExists => Some((StatusCode::CONFLICT, GROUP_ALREADY_EXISTS, None)),
        Errors::GroupIsSystem => Some((StatusCode::FORBIDDEN, GROUP_IS_SYSTEM, None)),
        Errors::GroupMemberNotFound => Some((StatusCode::NOT_FOUND, GROUP_MEMBER_NOT_FOUND, None)),
        Errors::GroupMemberAlreadyExists => {
            Some((StatusCode::CONFLICT, GROUP_MEMBER_ALREADY_EXISTS, None))
        }
        Errors::InvalidPermission(detail) => Some((
            StatusCode::BAD_REQUEST,
            PERMISSION_INVALID,
            Some(detail.clone()),
        )),
        _ => None,
    }
}
