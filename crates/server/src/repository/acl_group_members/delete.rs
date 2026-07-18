use entity::acl_group_members::{Column as AclGroupMemberColumn, Entity as AclGroupMemberEntity};
use errors::errors::Errors;
use ipnetwork::IpNetwork;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Deletes one member row by id. Returns whether a row was deleted.
pub async fn repository_delete_acl_group_member<C>(
    conn: &C,
    member_id: Uuid,
) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let result = AclGroupMemberEntity::delete_by_id(member_id)
        .exec(conn)
        .await?;
    Ok(result.rows_affected > 0)
}

/// Deletes every membership row (active or expired) for a user in a group.
pub async fn repository_delete_acl_group_members_for_user<C>(
    conn: &C,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = AclGroupMemberEntity::delete_many()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(AclGroupMemberColumn::UserId.eq(user_id))
        .exec(conn)
        .await?;
    Ok(result.rows_affected)
}

/// Deletes every membership row (active or expired) for an exact IP/CIDR in a group.
pub async fn repository_delete_acl_group_members_for_ip<C>(
    conn: &C,
    group_id: Uuid,
    ip: &IpNetwork,
) -> Result<u64, Errors>
where
    C: ConnectionTrait,
{
    let result = AclGroupMemberEntity::delete_many()
        .filter(AclGroupMemberColumn::GroupId.eq(group_id))
        .filter(AclGroupMemberColumn::IpAddress.eq(*ip))
        .exec(conn)
        .await?;
    Ok(result.rows_affected)
}
