use super::common::member_to_response;
use crate::permission::PermissionService;
use crate::repository::groups::repository_find_group_by_id;
use crate::repository::moderation::repository_create_moderation_log;
use crate::repository::user::repository_find_user_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::groups::membership::{self, Authority, GrantParams};
use chrono::Utc;
use constants::ModerationAction;
use dto::groups::{AddGroupMemberRequest, GroupMemberResponse};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::json;
use tracing::info;

/// Adds a user member to an ACL group.
///
/// # Role
/// - Admin only.
///
/// # System groups
/// - Refused (`Errors::GroupIsSystem`) via the membership chokepoint:
///   system groups carry code-known meaning and are not mutable through the
///   generic admin API.
///
/// # Errors
/// - Returns `Errors::InvalidPermission` when `expires_at` is in the past.
/// - Returns `Errors::GroupNotFound` / `Errors::GroupIsSystem` /
///   `Errors::UserNotFound` for invalid targets.
/// - Returns `Errors::GroupMemberAlreadyExists` for an active duplicate.
pub async fn service_add_group_member(
    db: &DatabaseConnection,
    payload: AddGroupMemberRequest,
    session: &SessionContext,
) -> ServiceResult<GroupMemberResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    if let Some(expires_at) = payload.expires_at
        && expires_at <= Utc::now()
    {
        return Err(Errors::InvalidPermission(
            "expires_at must be in the future".to_string(),
        ));
    }

    let txn = db.begin().await?;

    let group = repository_find_group_by_id(&txn, payload.group_id)
        .await?
        .ok_or(Errors::GroupNotFound)?;

    // Confirm the target user exists before granting.
    repository_find_user_by_id(&txn, payload.user_id)
        .await?
        .ok_or(Errors::UserNotFound)?;

    let member = membership::grant(
        &txn,
        &group,
        payload.user_id,
        GrantParams {
            reason: payload.reason.clone(),
            expires_at: payload.expires_at,
            created_by: session.user_id,
        },
        Authority::Generic,
        Errors::GroupMemberAlreadyExists,
    )
    .await?;

    repository_create_moderation_log(
        &txn,
        ModerationAction::GroupMemberAdd,
        Some(session.user_id),
        ModerationResourceType::Group,
        Some(group.id),
        payload.reason.unwrap_or_default(),
        Some(json!({
            "member_id": member.id,
            "user_id": member.user_id,
            "expires_at": member.expires_at,
        })),
    )
    .await?;

    txn.commit().await?;

    info!(group_id = %group.id, member_id = %member.id, actor_id = %session.user_id, "ACL group member added");

    Ok(member_to_response(member))
}
