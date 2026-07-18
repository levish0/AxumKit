use sea_orm::prelude::*;
use uuid::Uuid;

use super::users::Entity as UsersEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(not_null, unique)]
    pub user_id: Uuid,
    #[sea_orm(column_type = "Boolean", not_null, default_value = "false")]
    pub email_enabled: bool,
    #[sea_orm(column_type = "Boolean", not_null, default_value = "false")]
    pub push_enabled: bool,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "UsersEntity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<UsersEntity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
