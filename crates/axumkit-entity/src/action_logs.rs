use sea_orm::prelude::*;
use uuid::Uuid;

use super::common::ActionResourceType;
use super::users::Entity as UsersEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "action_logs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", not_null)]
    pub action: String,
    #[sea_orm(nullable)]
    pub actor_id: Option<Uuid>,
    #[sea_orm(nullable)]
    pub actor_ip: Option<IpNetwork>,
    #[sea_orm(not_null)]
    pub resource_type: ActionResourceType,
    #[sea_orm(nullable)]
    pub resource_id: Option<Uuid>,
    #[sea_orm(column_type = "Text", not_null)]
    pub summary: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metadata: Option<Json>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "UsersEntity",
        from = "Column::ActorId",
        to = "super::users::Column::Id",
        on_delete = "SetNull"
    )]
    Actor,
}

impl Related<UsersEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Actor.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
