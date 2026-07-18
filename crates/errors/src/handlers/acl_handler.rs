use crate::errors::Errors;
use crate::protocol::acl::*;
use axum::http::StatusCode;
use tracing::debug;

/// ACL domain error logging.
pub fn log_error(error: &Errors) {
    match error {
        Errors::AclDenied(_)
        | Errors::AclGroupNotFound
        | Errors::AclGroupAlreadyExists
        | Errors::AclGroupIsSystem
        | Errors::AclGroupMemberNotFound
        | Errors::AclGroupMemberAlreadyExists
        | Errors::AclInvalidRule(_) => {
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
        Errors::AclDenied(detail) => {
            Some((StatusCode::FORBIDDEN, ACL_DENIED, Some(detail.clone())))
        }
        Errors::AclGroupNotFound => Some((StatusCode::NOT_FOUND, ACL_GROUP_NOT_FOUND, None)),
        Errors::AclGroupAlreadyExists => {
            Some((StatusCode::CONFLICT, ACL_GROUP_ALREADY_EXISTS, None))
        }
        Errors::AclGroupIsSystem => Some((StatusCode::FORBIDDEN, ACL_GROUP_IS_SYSTEM, None)),
        Errors::AclGroupMemberNotFound => {
            Some((StatusCode::NOT_FOUND, ACL_GROUP_MEMBER_NOT_FOUND, None))
        }
        Errors::AclGroupMemberAlreadyExists => {
            Some((StatusCode::CONFLICT, ACL_GROUP_MEMBER_ALREADY_EXISTS, None))
        }
        Errors::AclInvalidRule(detail) => Some((
            StatusCode::BAD_REQUEST,
            ACL_INVALID_RULE,
            Some(detail.clone()),
        )),
        _ => None,
    }
}
