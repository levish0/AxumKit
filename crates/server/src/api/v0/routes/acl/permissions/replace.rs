use crate::extractors::RequiredSession;
use crate::service::acl::service_replace_acl_group_permissions;
use crate::state::AppState;
use axum::extract::State;
use dto::acl::{AclGroupPermissionsResponse, ReplaceAclGroupPermissionsRequest};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/acl/groups/permissions/replace",
    summary = "Replace a group's permissions",
    description = "Replaces a group's permission grants with the submitted list (whole-list replacement). Admin only.",
    request_body = ReplaceAclGroupPermissionsRequest,
    responses(
        (status = 200, description = "Group permissions replaced", body = AclGroupPermissionsResponse),
        (status = 400, description = "Bad request - Unknown permission codename", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn replace_acl_group_permissions(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<ReplaceAclGroupPermissionsRequest>,
) -> Result<AclGroupPermissionsResponse, Errors> {
    service_replace_acl_group_permissions(&state.db, payload, &session).await
}
