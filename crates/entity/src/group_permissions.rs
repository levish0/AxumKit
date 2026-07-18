use sea_orm::prelude::*;
use uuid::Uuid;

/// Permission granted to an ACL group (Django's `group_permissions`).
///
/// `permission` stores a `constants::Permission` codename as TEXT (e.g.
/// `board:pin_post`) — deliberately not a Postgres enum so new permissions
/// need no migration. A stored codename that no longer parses never matches
/// any check (logged, fail-closed).
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "group_permissions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    /// Owning group
    #[sea_orm(not_null)]
    pub group_id: Uuid,
    /// Permission codename (`constants::Permission` string form)
    #[sea_orm(column_type = "Text", not_null)]
    pub permission: String,
    /// Admin who granted the permission (SetNull on account deletion)
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
        belongs_to = "super::users::Entity",
        from = "Column::CreatedBy",
        to = "super::users::Column::Id",
        on_delete = "SetNull"
    )]
    CreatedByUser,
}

impl Related<super::groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
