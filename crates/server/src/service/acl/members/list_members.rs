use super::common::member_to_response;
use crate::permission::PermissionService;
use crate::repository::acl_group_members::repository_find_acl_group_members_paginated;
use crate::repository::acl_groups::repository_find_acl_group_by_id;
use crate::service::auth::session_types::SessionContext;
use dto::acl::{AclGroupMemberListResponse, ListAclGroupMembersRequest};
use entity::common::Role;
use errors::errors::{Errors, ServiceResult};
use sea_orm::DatabaseConnection;

/// Lists a group's active members with cursor pagination (newest first).
///
/// # Role
/// - `ModDocument` (or admin) only.
///
/// # Errors
/// - Returns `Errors::AclGroupNotFound` when the group does not exist.
pub async fn service_list_acl_group_members(
    db: &DatabaseConnection,
    payload: ListAclGroupMembersRequest,
    session: &SessionContext,
) -> ServiceResult<AclGroupMemberListResponse> {
    PermissionService::require_role(db, Some(session), Role::Mod).await?;

    let group = repository_find_acl_group_by_id(db, payload.group_id)
        .await?
        .ok_or(Errors::AclGroupNotFound)?;

    // Fetch one extra row to detect whether more pages exist.
    let limit = payload.limit;
    let mut members =
        repository_find_acl_group_members_paginated(db, group.id, payload.cursor_id, limit + 1)
            .await?;

    let has_more = members.len() as u64 > limit;
    members.truncate(limit as usize);

    Ok(AclGroupMemberListResponse {
        data: members.into_iter().map(member_to_response).collect(),
        has_more,
    })
}
