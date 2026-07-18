use crate::permission::PermissionService;
use crate::repository::acl_groups::repository_list_acl_groups;
use crate::service::auth::session_types::SessionContext;
use dto::acl::{AclGroupListResponse, AclGroupResponse};
use entity::common::Role;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// Lists every ACL group (name order).
///
/// # Role
/// - `ModDocument` (or admin) only.
pub async fn service_list_acl_groups(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<AclGroupListResponse> {
    PermissionService::require_role(db, Some(session), Role::Mod).await?;

    let groups = repository_list_acl_groups(db).await?;

    Ok(AclGroupListResponse {
        groups: groups
            .into_iter()
            .map(|group| AclGroupResponse {
                id: group.id,
                name: group.name,
                description: group.description,
                is_system: group.is_system,
                created_at: group.created_at,
            })
            .collect(),
    })
}
