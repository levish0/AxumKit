use super::common::member_to_response;
use crate::permission::PermissionService;
use crate::repository::groups::repository_find_group_by_id;
use crate::repository::moderation::repository_create_moderation_log;
use crate::repository::user::repository_find_user_by_id;
use crate::service::auth::session_types::SessionContext;
use crate::service::groups::membership::{self, Authority, GrantParams, MemberSubject};
use crate::utils::ip::canonicalize_ip_network;
use chrono::Utc;
use constants::ModerationAction;
use dto::groups::{AddGroupMemberRequest, GroupMemberResponse};
use entity::common::{ModerationResourceType, Role};
use errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait, prelude::IpNetwork};
use serde_json::json;
use tracing::info;

/// Adds a member (a user XOR an IP/CIDR) to an ACL group.
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
/// - Returns `Errors::InvalidPermission` unless exactly one subject is given or
///   when `expires_at` is in the past.
/// - Returns `Errors::GroupNotFound` / `Errors::GroupIsSystem` /
///   `Errors::UserNotFound` / `Errors::InvalidIpAddress` for invalid targets.
/// - Returns `Errors::GroupMemberAlreadyExists` for an active duplicate.
pub async fn service_add_group_member(
    db: &DatabaseConnection,
    payload: AddGroupMemberRequest,
    session: &SessionContext,
) -> ServiceResult<GroupMemberResponse> {
    PermissionService::require_role(db, Some(session), Role::Admin).await?;

    // Exactly one subject: a user or an IP/CIDR (mirrors the DB CHECK).
    if payload.user_id.is_some() == payload.ip_address.is_some() {
        return Err(Errors::InvalidPermission(
            "exactly one of user_id or ip_address must be provided".to_string(),
        ));
    }

    if let Some(expires_at) = payload.expires_at
        && expires_at <= Utc::now()
    {
        return Err(Errors::InvalidPermission(
            "expires_at must be in the future".to_string(),
        ));
    }

    // Canonicalize the IP subject so stored members match the canonical client
    // IP produced at lookup time.
    let subject = match (payload.user_id, payload.ip_address.as_deref()) {
        (Some(user_id), None) => MemberSubject::User(user_id),
        (None, Some(raw)) => {
            let ip = raw
                .parse::<IpNetwork>()
                .map_err(|_| Errors::InvalidIpAddress)?;
            MemberSubject::Ip(canonicalize_ip_network(ip))
        }
        _ => unreachable!("subject count validated above"),
    };

    let txn = db.begin().await?;

    let group = repository_find_group_by_id(&txn, payload.group_id)
        .await?
        .ok_or(Errors::GroupNotFound)?;

    // Confirm a user subject exists before granting (an IP subject has nothing
    // to look up).
    if let MemberSubject::User(user_id) = subject {
        repository_find_user_by_id(&txn, user_id)
            .await?
            .ok_or(Errors::UserNotFound)?;
    }

    let member = membership::grant(
        &txn,
        &group,
        subject,
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
            "ip_address": member.ip_address.map(|ip| ip.to_string()),
            "expires_at": member.expires_at,
        })),
    )
    .await?;

    txn.commit().await?;

    info!(group_id = %group.id, member_id = %member.id, actor_id = %session.user_id, "ACL group member added");

    Ok(member_to_response(member))
}
