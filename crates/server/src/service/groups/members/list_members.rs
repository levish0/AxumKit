use super::common::member_to_response;
use crate::permission::PermissionService;
use crate::repository::group_members::repository_find_group_members_paginated;
use crate::repository::groups::repository_find_group_by_id;
use crate::service::auth::session_types::SessionContext;
use dto::groups::{GroupMemberListResponse, ListGroupMembersRequest};
use entity::common::Role;
use errors::errors::{Errors, ServiceResult};
use sea_orm::DatabaseConnection;

/// Lists a group's active members with cursor pagination (newest first).
///
/// # Role
/// - `ModDocument` (or admin) only.
///
/// # Errors
/// - Returns `Errors::GroupNotFound` when the group does not exist.
pub async fn service_list_group_members(
    db: &DatabaseConnection,
    payload: ListGroupMembersRequest,
    session: &SessionContext,
) -> ServiceResult<GroupMemberListResponse> {
    PermissionService::require_role(db, Some(session), Role::Mod).await?;

    let group = repository_find_group_by_id(db, payload.group_id)
        .await?
        .ok_or(Errors::GroupNotFound)?;

    // Fetch one extra row to detect whether more pages exist.
    let limit = payload.limit;
    let mut members =
        repository_find_group_members_paginated(db, group.id, payload.cursor_id, limit + 1).await?;

    let has_more = members.len() as u64 > limit;
    members.truncate(limit as usize);

    Ok(GroupMemberListResponse {
        data: members.into_iter().map(member_to_response).collect(),
        has_more,
    })
}
