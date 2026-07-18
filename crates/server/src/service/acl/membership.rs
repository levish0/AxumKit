//! The single chokepoint for ACL group-membership mutations.
//!
//! Every path that adds or removes a member — the user/IP ban and unban
//! services and the generic ACL member API — funnels through here, so the
//! system-group policy and the upsert/revoke mechanics live in exactly one
//! place instead of being copy-pasted per caller. Callers keep only what is
//! genuinely theirs: authorization, moderation logging, and the response shape.

use chrono::{DateTime, Utc};
use entity::acl_group_members::Model as AclGroupMemberModel;
use entity::acl_groups::Model as AclGroupModel;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, prelude::IpNetwork};
use uuid::Uuid;

use crate::repository::acl_group_members::{
    AclGroupMemberCreateParams, repository_create_acl_group_member,
    repository_delete_acl_group_member, repository_delete_acl_group_members_for_ip,
    repository_delete_acl_group_members_for_user, repository_find_active_acl_group_member_for_ip,
    repository_find_active_acl_group_member_for_user,
};

/// A membership subject: a user or an IP/CIDR — exactly one, mirroring the DB
/// CHECK on `acl_group_members`.
#[derive(Clone, Copy)]
pub(crate) enum MemberSubject {
    User(Uuid),
    Ip(IpNetwork),
}

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

/// Adds `subject` to `group`, upsert-style: rejects an active duplicate (as
/// `duplicate_error`), clears any stale rows so the new one is the only row,
/// and inserts. Runs inside the caller's transaction.
pub(crate) async fn grant<C: ConnectionTrait>(
    conn: &C,
    group: &AclGroupModel,
    subject: MemberSubject,
    params: GrantParams,
    authority: Authority,
    duplicate_error: Errors,
) -> Result<AclGroupMemberModel, Errors> {
    enforce_system_policy(group, authority)?;

    if find_active(conn, group.id, subject).await?.is_some() {
        return Err(duplicate_error);
    }
    delete_for_subject(conn, group.id, subject).await?;

    let (user_id, ip_address) = subject.split();
    repository_create_acl_group_member(
        conn,
        AclGroupMemberCreateParams {
            group_id: group.id,
            user_id,
            ip_address,
            reason: params.reason,
            expires_at: params.expires_at,
            created_by: Some(params.created_by),
        },
    )
    .await
}

/// Removes a single, already-loaded membership row by id (the by-member-id
/// path: generic remove and IP unban). Enforces the system-group policy.
pub(crate) async fn revoke_row<C: ConnectionTrait>(
    conn: &C,
    group: &AclGroupModel,
    member: &AclGroupMemberModel,
    authority: Authority,
) -> Result<(), Errors> {
    enforce_system_policy(group, authority)?;
    repository_delete_acl_group_member(conn, member.id).await?;
    Ok(())
}

/// The one place the system-group rule is enforced.
fn enforce_system_policy(group: &AclGroupModel, authority: Authority) -> Result<(), Errors> {
    if matches!(authority, Authority::Generic) && group.is_system {
        return Err(Errors::AclGroupIsSystem);
    }
    Ok(())
}

impl MemberSubject {
    fn split(self) -> (Option<Uuid>, Option<IpNetwork>) {
        match self {
            MemberSubject::User(user_id) => (Some(user_id), None),
            MemberSubject::Ip(ip) => (None, Some(ip)),
        }
    }
}

async fn find_active<C: ConnectionTrait>(
    conn: &C,
    group_id: Uuid,
    subject: MemberSubject,
) -> Result<Option<AclGroupMemberModel>, Errors> {
    match subject {
        MemberSubject::User(user_id) => {
            repository_find_active_acl_group_member_for_user(conn, group_id, user_id).await
        }
        MemberSubject::Ip(ip) => {
            repository_find_active_acl_group_member_for_ip(conn, group_id, &ip).await
        }
    }
}

async fn delete_for_subject<C: ConnectionTrait>(
    conn: &C,
    group_id: Uuid,
    subject: MemberSubject,
) -> Result<(), Errors> {
    match subject {
        MemberSubject::User(user_id) => {
            repository_delete_acl_group_members_for_user(conn, group_id, user_id).await?;
        }
        MemberSubject::Ip(ip) => {
            repository_delete_acl_group_members_for_ip(conn, group_id, &ip).await?;
        }
    }
    Ok(())
}
