use crate::permission::PermissionService;
use crate::repository::groups::{repository_create_group, repository_find_group_by_name};
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use constants::ModerationAction;
use dto::groups::{CreateGroupRequest, GroupResponse};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// Creates a (non-system) ACL group.
///
/// # Role
/// - Admin only.
///
/// # Errors
/// - Returns `Errors::GroupAlreadyExists` when the name is taken.
pub async fn service_create_group(
    db: &DatabaseConnection,
    payload: CreateGroupRequest,
    session: &SessionContext,
) -> ServiceResult<GroupResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    let txn = db.begin().await?;

    if repository_find_group_by_name(&txn, &payload.name)
        .await?
        .is_some()
    {
        return Err(Errors::GroupAlreadyExists);
    }

    let group = repository_create_group(&txn, payload.name, payload.description).await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::GroupCreate,
        Some(session.user_id),
        ModerationResourceType::Group,
        Some(group.id),
        payload.reason,
        Some(json!({ "name": group.name })),
    )
    .await?;

    txn.commit().await?;

    info!(group_id = %group.id, name = %group.name, actor_id = %session.user_id, "ACL group created");

    Ok(GroupResponse {
        id: group.id,
        name: group.name,
        description: group.description,
        is_system: group.is_system,
        created_at: group.created_at,
    })
}
