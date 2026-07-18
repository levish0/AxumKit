use sea_orm::prelude::*;
use uuid::Uuid;

/// ACL group — a named set of subjects (users/IPs) that carries a bundle of
/// granted permissions (Django-style RBAC group).
///
/// `Role` is the coarse capability axis (`Mod`/`Admin`); groups are the
/// fine-grained one: membership in a group grants every permission attached to
/// it via `group_permissions`. System groups cannot be deleted.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "groups")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    /// Group identifier (unique)
    #[sea_orm(column_type = "Text", unique)]
    pub name: String,
    /// Group description
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    /// Whether this is a system group — one whose meaning the code knows
    /// (cannot be deleted)
    pub is_system: bool,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::group_members::Entity")]
    Members,
    #[sea_orm(has_many = "super::group_permissions::Entity")]
    Permissions,
}

impl Related<super::group_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Members.def()
    }
}

impl Related<super::group_permissions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Permissions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
