//! The single chokepoint for ACL group-membership mutations.
//!
//! Every path that adds or removes a member funnels through here, so the
//! system-group policy and the upsert/revoke mechanics live in exactly one
//! place instead of being copy-pasted per caller. Callers keep only what is
//! genuinely theirs: authorization, moderation logging, and the response shape.

use chrono::{DateTime, Utc};
use entity::group_members::Model as GroupMemberModel;
use entity::groups::Model as GroupModel;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use crate::repository::group_members::{
    GroupMemberCreateParams, repository_create_group_member, repository_delete_group_member,
    repository_delete_group_members_for_user, repository_find_active_group_member_for_user,
};

/// Who is mutating the membership. System groups carry code-known meaning and
/// are refused to the generic ACL admin API (`Generic`) — a future privileged
/// path would add its own authority variant.
#[derive(Clone, Copy)]
pub(crate) enum Authority {
    Generic,
}

/// Metadata for a new membership row.
pub(crate) struct GrantParams {
    pub reason: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
}

/// Adds `user_id` to `group`, upsert-style: rejects an active duplicate (as
/// `duplicate_error`), clears any stale rows so the new one is the only row,
/// and inserts. Runs inside the caller's transaction.
pub(crate) async fn grant<C: ConnectionTrait>(
    conn: &C,
    group: &GroupModel,
    user_id: Uuid,
    params: GrantParams,
    authority: Authority,
    duplicate_error: Errors,
) -> Result<GroupMemberModel, Errors> {
    enforce_system_policy(group, authority)?;

    if repository_find_active_group_member_for_user(conn, group.id, user_id)
        .await?
        .is_some()
    {
        return Err(duplicate_error);
    }
    repository_delete_group_members_for_user(conn, group.id, user_id).await?;

    repository_create_group_member(
        conn,
        GroupMemberCreateParams {
            group_id: group.id,
            user_id,
            reason: params.reason,
            expires_at: params.expires_at,
            created_by: Some(params.created_by),
        },
    )
    .await
}

/// Removes a single, already-loaded membership row by id. Enforces the
/// system-group policy.
pub(crate) async fn revoke_row<C: ConnectionTrait>(
    conn: &C,
    group: &GroupModel,
    member: &GroupMemberModel,
    authority: Authority,
) -> Result<(), Errors> {
    enforce_system_policy(group, authority)?;
    repository_delete_group_member(conn, member.id).await?;
    Ok(())
}

/// The one place the system-group rule is enforced.
fn enforce_system_policy(group: &GroupModel, authority: Authority) -> Result<(), Errors> {
    if matches!(authority, Authority::Generic) && group.is_system {
        return Err(Errors::GroupIsSystem);
    }
    Ok(())
}
