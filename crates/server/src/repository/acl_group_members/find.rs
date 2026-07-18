use chrono::Utc;
use entity::acl_group_members::{
    Column as AclGroupMemberColumn, Entity as AclGroupMemberEntity, Model as AclGroupMemberModel,
};
use entity::acl_groups::{Entity as AclGroupEntity, Model as AclGroupModel};
use errors::errors::Errors;
use sea_orm::sea_query::{BinOper, Expr};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder,
};
use std::net::IpAddr;
use uuid::Uuid;

/// Finds the subject's active memberships (matched by user id and/or by IP
/// containment for CIDR members), joined with their groups.
pub async fn repository_find_active_acl_group_memberships<C>(
    conn: &C,
    user_id: Option<Uuid>,
    ip: Option<&IpAddr>,
) -> Result<Vec<(AclGroupMemberModel, Option<AclGroupModel>)>, Errors>
where
    C: ConnectionTrait,
{
    if user_id.is_none() && ip.is_none() {
        return Ok(vec![]);
    }

    let mut subject = Condition::any();
    if let Some(user_id) = user_id {
        subject = subject.add(AclGroupMemberColumn::UserId.eq(user_id));
    }
    if let Some(ip) = ip {
        subject = subject.add(Expr::col(AclGroupMemberColumn::IpAddress).binary(
            BinOper::Custom(">>="),
            Expr::val(ip.to_string()).cast_as("inet"),
        ));
    }

    Ok(AclGroupMemberEntity::find()
        .find_also_related(AclGroupEntity)
        .filter(active_condition())
        .filter(subject)
        .all(conn)
        .await?)
}

/// Finds one member row by id.
pub async fn repository_find_acl_group_member_by_id<C>(
    conn: &C,
    member_id: Uuid,
) -> Result<Option<AclGroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupMemberEntity::find_by_id(member_id)
        .one(conn)
        .await?)
}

/// Finds an active membership row for a specific user in a specific group.
pub async fn repository_find_active_acl_group_member_for_user<C>(
    conn: &C,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<Option<AclGroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupMemberEntity::find()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(AclGroupMemberColumn::UserId.eq(user_id))
        .filter(active_condition())
        .one(conn)
        .await?)
}

/// Finds an active membership row for an exact IP/CIDR in a specific group.
pub async fn repository_find_active_acl_group_member_for_ip<C>(
    conn: &C,
    group_id: Uuid,
    ip: &ipnetwork::IpNetwork,
) -> Result<Option<AclGroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupMemberEntity::find()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(AclGroupMemberColumn::IpAddress.eq(*ip))
        .filter(active_condition())
        .one(conn)
        .await?)
}

/// Finds active membership rows for a set of users in a specific group
/// (batch ban-status lookups).
pub async fn repository_find_active_acl_group_members_for_users<C>(
    conn: &C,
    group_id: Uuid,
    user_ids: &[Uuid],
) -> Result<Vec<AclGroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    if user_ids.is_empty() {
        return Ok(vec![]);
    }

    Ok(AclGroupMemberEntity::find()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(AclGroupMemberColumn::UserId.is_in(user_ids.iter().copied()))
        .filter(active_condition())
        .all(conn)
        .await?)
}

/// Lists a group's active members with cursor pagination (newest first).
pub async fn repository_find_acl_group_members_paginated<C>(
    conn: &C,
    group_id: Uuid,
    cursor_id: Option<Uuid>,
    limit: u64,
) -> Result<Vec<AclGroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    use sea_orm::QuerySelect;

    let mut query = AclGroupMemberEntity::find()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(active_condition());

    if let Some(id) = cursor_id {
        query = query.filter(AclGroupMemberColumn::Id.lt(id));
    }

    Ok(query
        .order_by_desc(AclGroupMemberColumn::Id)
        .limit(limit)
        .all(conn)
        .await?)
}

/// Active membership predicate: permanent or not yet expired.
pub(crate) fn active_condition() -> Condition {
    let now = Utc::now();
    Condition::any()
        .add(AclGroupMemberColumn::ExpiresAt.is_null())
        .add(AclGroupMemberColumn::ExpiresAt.gt(now))
}
