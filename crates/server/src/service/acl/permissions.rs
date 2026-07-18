//! Group permission-grant administration (Django's group-permissions editing).

use crate::permission::PermissionService;
use crate::repository::acl_group_permissions::{
    repository_find_permissions_for_group, repository_replace_acl_group_permissions,
};
use crate::repository::acl_groups::repository_find_acl_group_by_id;
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use constants::{ModerationAction, Permission};
use dto::acl::{
    AclGroupPermissionsResponse, AclPermissionListResponse, ReplaceAclGroupPermissionsRequest,
};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// Lists every permission codename the application defines — what an admin UI
/// offers as checkboxes.
///
/// # Role
/// - Mod or above (same bar as listing groups).
pub async fn service_list_permissions(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<AclPermissionListResponse> {
    PermissionService::require_role(db, Some(session), Role::Mod).await?;

    Ok(AclPermissionListResponse {
        permissions: Permission::ALL
            .iter()
            .map(|p| p.as_str().to_string())
            .collect(),
    })
}

/// Lists a group's granted permissions.
///
/// # Role
/// - Mod or above.
pub async fn service_get_acl_group_permissions(
    db: &DatabaseConnection,
    group_id: uuid::Uuid,
    session: &SessionContext,
) -> ServiceResult<AclGroupPermissionsResponse> {
    PermissionService::require_role(db, Some(session), Role::Mod).await?;

    let group = repository_find_acl_group_by_id(db, group_id)
        .await?
        .ok_or(Errors::AclGroupNotFound)?;
    let grants = repository_find_permissions_for_group(db, group.id).await?;

    Ok(AclGroupPermissionsResponse {
        group_id: group.id,
        permissions: grants.into_iter().map(|g| g.permission).collect(),
    })
}

/// Replaces a group's permission grants with the submitted list (whole-list
/// replacement: list state is the API contract).
///
/// # Role
/// - Admin only.
///
/// # Errors
/// - `Errors::AclInvalidRule` for a codename the application does not define
///   (typos must not become silent dead grants).
pub async fn service_replace_acl_group_permissions(
    db: &DatabaseConnection,
    payload: ReplaceAclGroupPermissionsRequest,
    session: &SessionContext,
) -> ServiceResult<AclGroupPermissionsResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    // Validate every codename before touching the DB.
    let mut permissions: Vec<String> = Vec::with_capacity(payload.permissions.len());
    for raw in &payload.permissions {
        let permission = raw
            .parse::<Permission>()
            .map_err(|_| Errors::AclInvalidRule(format!("unknown permission: {raw}")))?;
        let canonical = permission.as_str().to_string();
        if !permissions.contains(&canonical) {
            permissions.push(canonical);
        }
    }

    let txn = db.begin().await?;

    let group = repository_find_acl_group_by_id(&txn, payload.group_id)
        .await?
        .ok_or(Errors::AclGroupNotFound)?;

    repository_replace_acl_group_permissions(&txn, group.id, &permissions, Some(session.user_id))
        .await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::AclGroupPermissionsReplace,
        Some(session.user_id),
        ModerationResourceType::AclGroup,
        Some(group.id),
        payload.reason,
        Some(json!({ "name": group.name, "permissions": permissions })),
    )
    .await?;

    txn.commit().await?;

    info!(
        group_id = %group.id,
        count = permissions.len(),
        actor_id = %session.user_id,
        "ACL group permissions replaced"
    );

    Ok(AclGroupPermissionsResponse {
        group_id: group.id,
        permissions,
    })
}
