use super::common::member_to_response;
use crate::permission::PermissionService;
use crate::repository::group_members::repository_find_group_member_by_id;
use crate::repository::groups::repository_find_group_by_id;
use crate::repository::moderation::repository_create_moderation_log;
use crate::service::auth::session_types::SessionContext;
use crate::service::groups::membership::{self, Authority};
use constants::ModerationAction;
use dto::groups::{GroupMemberResponse, RemoveGroupMemberRequest};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// Removes a member row from an ACL group.
///
/// # Role
/// - Admin only.
///
/// # System groups
/// - Refused (`Errors::GroupIsSystem`) via the membership chokepoint, so a
///   ban cannot be lifted outside the audited unban flow.
///
/// # Errors
/// - Returns `Errors::GroupMemberNotFound` when the row does not exist.
/// - Returns `Errors::GroupIsSystem` for a system-group membership.
pub async fn service_remove_group_member(
    db: &DatabaseConnection,
    payload: RemoveGroupMemberRequest,
    session: &SessionContext,
) -> ServiceResult<GroupMemberResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    let txn = db.begin().await?;

    let member = repository_find_group_member_by_id(&txn, payload.member_id)
        .await?
        .ok_or(Errors::GroupMemberNotFound)?;
    let group = repository_find_group_by_id(&txn, member.group_id)
        .await?
        .ok_or(Errors::GroupNotFound)?;

    membership::revoke_row(&txn, &group, &member, Authority::Generic).await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::GroupMemberRemove,
        Some(session.user_id),
        ModerationResourceType::Group,
        Some(member.group_id),
        payload.reason,
        Some(json!({
            "member_id": member.id,
            "user_id": member.user_id,
            "ip_address": member.ip_address.map(|ip| ip.to_string()),
        })),
    )
    .await?;

    txn.commit().await?;

    info!(group_id = %member.group_id, member_id = %member.id, actor_id = %session.user_id, "ACL group member removed");

    Ok(member_to_response(member))
}
