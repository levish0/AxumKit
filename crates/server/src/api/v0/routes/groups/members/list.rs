use crate::extractors::RequiredSession;
use crate::service::groups::service_list_group_members;
use crate::state::AppState;
use axum::extract::State;
use dto::groups::{GroupMemberListResponse, ListGroupMembersRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::{ErrorResponse, Errors};

#[utoipa::path(
    get,
    path = "/v0/groups/members",
    summary = "List ACL group members",
    description = "Returns a group's active members with cursor pagination (newest first). Requires the document moderator role.",
    params(ListGroupMembersRequest),
    responses(
        (status = 200, description = "ACL group members retrieved successfully", body = GroupMemberListResponse),
        (status = 401, description = "Unauthorized - Login required", body = ErrorResponse),
        (status = 403, description = "Forbidden - Insufficient permissions", body = ErrorResponse),
        (status = 404, description = "Not Found - Group not found", body = ErrorResponse),
        (status = 500, description = "Internal Server Error - Database error", body = ErrorResponse)
    ),
    security(
        ("session_id_cookie" = [])
    ),
    tag = "ACL"
)]
pub async fn list_group_members(
    State(state): State<AppState>,
    RequiredSession(session): RequiredSession,
    ValidatedQuery(payload): ValidatedQuery<ListGroupMembersRequest>,
) -> Result<GroupMemberListResponse, Errors> {
    service_list_group_members(&state.db, payload, &session).await
}
