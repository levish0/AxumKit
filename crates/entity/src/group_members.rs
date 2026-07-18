use sea_orm::prelude::*;
use uuid::Uuid;

use super::users::Entity as UsersEntity;

/// ACL group member.
///
/// Expiry is read-time filtering (`expires_at > now`), not deletion.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "group_members")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    /// Owning group
    #[sea_orm(not_null)]
    pub group_id: Uuid,
    /// User member
    #[sea_orm(not_null)]
    pub user_id: Uuid,
    /// Membership reason
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    /// Expiry (None = permanent)
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub expires_at: Option<DateTimeUtc>,
    /// Admin who added the member (NULL when that admin is deleted)
    #[sea_orm(nullable)]
    pub created_by: Option<Uuid>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::groups::Entity",
        from = "Column::GroupId",
        to = "super::groups::Column::Id",
        on_delete = "Cascade"
    )]
    Group,
    #[sea_orm(
        belongs_to = "UsersEntity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "UsersEntity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id",
        on_delete = "SetNull"
    )]
    Creator,
}

impl Related<super::groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
