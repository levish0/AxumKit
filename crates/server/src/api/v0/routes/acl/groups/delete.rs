use crate::extractors::RequiredSession;
use crate::service::acl::service_delete_acl_group;
use crate::state::AppState;
use axum::extract::State;
use dto::acl::{AclGroupResponse, DeleteAclGroupRequest};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/acl/groups/delete",
    summary = "Delete an ACL group",
    description = "Deletes a non-system ACL group that no rules reference. Admin only.",
    request_body = DeleteAclGroupRequest,
    responses(
        (status = 200, description = "ACL group deleted successfully", body = AclGroupResponse),
        (status = 400, description = "Bad request - System group or still referenced by rules", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn delete_acl_group(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<DeleteAclGroupRequest>,
) -> Result<AclGroupResponse, Errors> {
    service_delete_acl_group(&state.db, payload, &session).await
}
