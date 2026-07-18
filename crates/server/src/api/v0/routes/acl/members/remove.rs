use crate::extractors::RequiredSession;
use crate::service::acl::service_remove_acl_group_member;
use crate::state::AppState;
use axum::extract::State;
use dto::acl::{AclGroupMemberResponse, RemoveAclGroupMemberRequest};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/acl/groups/members/remove",
    summary = "Remove an ACL group member",
    description = "Removes a membership row from an ACL group. Admin only.",
    request_body = RemoveAclGroupMemberRequest,
    responses(
        (status = 200, description = "ACL group member removed successfully", body = AclGroupMemberResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Member not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn remove_acl_group_member(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<RemoveAclGroupMemberRequest>,
) -> Result<AclGroupMemberResponse, Errors> {
    service_remove_acl_group_member(&state.db, payload, &session).await
}
