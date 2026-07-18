use chrono::{DateTime, Utc};
use entity::group_members::{ActiveModel as GroupMemberActiveModel, Model as GroupMemberModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

/// Parameters for adding a user member to an ACL group.
#[derive(Debug, Clone)]
pub struct GroupMemberCreateParams {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub reason: Option<String>,
    /// None = permanent membership.
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// Adds a member row to an ACL group.
pub async fn repository_create_group_member<C>(
    conn: &C,
    params: GroupMemberCreateParams,
) -> Result<GroupMemberModel, Errors>
where
    C: ConnectionTrait,
{
    let member = GroupMemberActiveModel {
        group_id: Set(params.group_id),
        user_id: Set(params.user_id),
        reason: Set(params.reason),
        expires_at: Set(params.expires_at),
        created_by: Set(params.created_by),
        ..Default::default()
    };

    Ok(member.insert(conn).await?)
}
