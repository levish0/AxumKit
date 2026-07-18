use crate::permission::PermissionService;
use crate::repository::groups::{repository_delete_group, repository_find_group_by_id};
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use constants::ModerationAction;
use dto::groups::{DeleteGroupRequest, GroupResponse};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// Deletes an ACL group. Members cascade at the DB level.
///
/// # Role
/// - Admin only.
///
/// # Errors
/// - Returns `Errors::GroupNotFound` when the group does not exist.
/// - Returns `Errors::GroupIsSystem` for system groups.
/// - Returns `Errors::InvalidPermission` while rules still reference the group
///   (the FK is RESTRICT — rules must be detached first).
pub async fn service_delete_group(
    db: &DatabaseConnection,
    payload: DeleteGroupRequest,
    session: &SessionContext,
) -> ServiceResult<GroupResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    let txn = db.begin().await?;

    let group = repository_find_group_by_id(&txn, payload.group_id)
        .await?
        .ok_or(Errors::GroupNotFound)?;

    if group.is_system {
        return Err(Errors::GroupIsSystem);
    }

    // Permission grants and memberships cascade with the group row.
    repository_delete_group(&txn, group.id).await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::GroupDelete,
        Some(session.user_id),
        ModerationResourceType::Group,
        Some(group.id),
        payload.reason,
        Some(json!({ "name": group.name })),
    )
    .await?;

    txn.commit().await?;

    info!(group_id = %group.id, name = %group.name, actor_id = %session.user_id, "ACL group deleted");

    Ok(GroupResponse {
        id: group.id,
        name: group.name,
        description: group.description,
        is_system: group.is_system,
        created_at: group.created_at,
    })
}
