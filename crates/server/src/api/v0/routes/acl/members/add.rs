use crate::extractors::RequiredSession;
use crate::service::acl::service_add_acl_group_member;
use crate::state::AppState;
use axum::extract::State;
use dto::acl::{AclGroupMemberResponse, AddAclGroupMemberRequest};
use dto::validator::json_validator::ValidatedJson;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    post,
    path = "/v0/acl/groups/members",
    summary = "Add an ACL group member",
    description = "Adds a user or an IP/CIDR range to an ACL group. Admin only.",
    request_body = AddAclGroupMemberRequest,
    responses(
        (status = 200, description = "ACL group member added successfully", body = AclGroupMemberResponse),
        (status = 400, description = "Bad request - Invalid subject, IP format, or expiry", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group or user not found", body = ErrorResponse),
        (status = 409, description = "Conflict - Member already exists", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database or transaction error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn add_acl_group_member(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedJson(payload): ValidatedJson<AddAclGroupMemberRequest>,
) -> Result<AclGroupMemberResponse, Errors> {
    service_add_acl_group_member(&state.db, payload, &session).await
}
