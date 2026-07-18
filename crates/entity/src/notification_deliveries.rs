use sea_orm::prelude::*;
use uuid::Uuid;

use super::notification_events::Entity as NotificationEventsEntity;
use super::users::Entity as UsersEntity;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "notification_deliveries")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(not_null)]
    pub user_id: Uuid,
    #[sea_orm(not_null)]
    pub event_id: Uuid,
    #[sea_orm(column_type = "Boolean", not_null, default_value = "false")]
    pub is_read: bool,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub read_at: Option<DateTimeUtc>,
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
    #[sea_orm(
        belongs_to = "NotificationEventsEntity",
        from = "Column::EventId",
        to = "super::notification_events::Column::Id",
        on_delete = "Cascade"
    )]
    Event,
}

impl Related<UsersEntity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<NotificationEventsEntity> for Entity {
    fn to() -> RelationDef {
        Relation::Event.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
