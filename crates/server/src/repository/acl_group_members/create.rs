use chrono::{DateTime, Utc};
use entity::acl_group_members::{
    ActiveModel as AclGroupMemberActiveModel, Model as AclGroupMemberModel,
};
use errors::errors::Errors;
use ipnetwork::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

/// Parameters for adding a member (a user XOR an IP/CIDR) to an ACL group.
#[derive(Debug, Clone)]
pub struct AclGroupMemberCreateParams {
    pub group_id: Uuid,
    pub user_id: Option<Uuid>,
    pub ip_address: Option<IpNetwork>,
    pub reason: Option<String>,
    /// None = permanent membership.
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// Adds a member row to an ACL group. The DB CHECK enforces exactly one
/// subject (user or IP) per row.
pub async fn repository_create_acl_group_member<C>(
    conn: &C,
    params: AclGroupMemberCreateParams,
) -> Result<AclGroupMemberModel, Errors>
where
    C: ConnectionTrait,
{
    let member = AclGroupMemberActiveModel {
        group_id: Set(params.group_id),
        user_id: Set(params.user_id),
        ip_address: Set(params.ip_address),
        reason: Set(params.reason),
        expires_at: Set(params.expires_at),
        created_by: Set(params.created_by),
        ..Default::default()
    };

    Ok(member.insert(conn).await?)
}
