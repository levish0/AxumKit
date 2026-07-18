use chrono::Utc;
use entity::group_members::{
    Column as GroupMemberColumn, Entity as GroupMemberEntity, Model as GroupMemberModel,
};
use entity::groups::{Entity as GroupEntity, Model as GroupModel};
use errors::errors::Errors;
use sea_orm::sea_query::{BinOper, Expr};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder,
};
use std::net::IpAddr;
use uuid::Uuid;

/// Finds the subject's active memberships (matched by user id and/or by IP
/// containment for CIDR members), joined with their groups.
pub async fn repository_find_active_group_memberships<C>(
    conn: &C,
    user_id: Option<Uuid>,
    ip: Option<&IpAddr>,
) -> Result<Vec<(GroupMemberModel, Option<GroupModel>)>, Errors>
where
    C: ConnectionTrait,
{
    if user_id.is_none() && ip.is_none() {
        return Ok(vec![]);
    }

    let mut subject = Condition::any();
    if let Some(user_id) = user_id {
        subject = subject.add(GroupMemberColumn::UserId.eq(user_id));
    }
    if let Some(ip) = ip {
        subject = subject.add(Expr::col(GroupMemberColumn::IpAddress).binary(
            BinOper::Custom(">>="),
            Expr::val(ip.to_string()).cast_as("inet"),
        ));
    }

    Ok(GroupMemberEntity::find()
        .find_also_related(GroupEntity)
        .filter(active_condition())
        .filter(subject)
        .all(conn)
        .await?)
}

/// Finds one member row by id.
pub async fn repository_find_group_member_by_id<C>(
    conn: &C,
    member_id: Uuid,
) -> Result<Option<GroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupMemberEntity::find_by_id(member_id).one(conn).await?)
}

/// Finds an active membership row for a specific user in a specific group.
pub async fn repository_find_active_group_member_for_user<C>(
    conn: &C,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<Option<GroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupMemberEntity::find()
        .filter(GroupMemberColumn::GroupId.eq(group_id))
        .filter(GroupMemberColumn::UserId.eq(user_id))
        .filter(active_condition())
        .one(conn)
        .await?)
}

/// Finds an active membership row for an exact IP/CIDR in a specific group.
pub async fn repository_find_active_group_member_for_ip<C>(
    conn: &C,
    group_id: Uuid,
    ip: &ipnetwork::IpNetwork,
) -> Result<Option<GroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupMemberEntity::find()
        .filter(GroupMemberColumn::GroupId.eq(group_id))
        .filter(GroupMemberColumn::IpAddress.eq(*ip))
        .filter(active_condition())
        .one(conn)
        .await?)
}

/// Finds active membership rows for a set of users in a specific group
/// (batch ban-status lookups).
pub async fn repository_find_active_group_members_for_users<C>(
    conn: &C,
    group_id: Uuid,
    user_ids: &[Uuid],
) -> Result<Vec<GroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    if user_ids.is_empty() {
        return Ok(vec![]);
    }

    Ok(GroupMemberEntity::find()
        .filter(GroupMemberColumn::GroupId.eq(group_id))
        .filter(GroupMemberColumn::UserId.is_in(user_ids.iter().copied()))
        .filter(active_condition())
        .all(conn)
        .await?)
}

/// Lists a group's active members with cursor pagination (newest first).
pub async fn repository_find_group_members_paginated<C>(
    conn: &C,
    group_id: Uuid,
    cursor_id: Option<Uuid>,
    limit: u64,
) -> Result<Vec<GroupMemberModel>, Errors>
where
    C: ConnectionTrait,
{
    use sea_orm::QuerySelect;

    let mut query = GroupMemberEntity::find()
        .filter(GroupMemberColumn::GroupId.eq(group_id))
        .filter(active_condition());

    if let Some(id) = cursor_id {
        query = query.filter(GroupMemberColumn::Id.lt(id));
    }

    Ok(query
        .order_by_desc(GroupMemberColumn::Id)
        .limit(limit)
        .all(conn)
        .await?)
}

/// Active membership predicate: permanent or not yet expired.
pub(crate) fn active_condition() -> Condition {
    let now = Utc::now();
    Condition::any()
        .add(GroupMemberColumn::ExpiresAt.is_null())
        .add(GroupMemberColumn::ExpiresAt.gt(now))
}
