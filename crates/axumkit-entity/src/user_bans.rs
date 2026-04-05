use sea_orm::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "user_bans")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(not_null, unique)]
    pub user_id: Uuid,
    #[sea_orm(column_type = "TimestampWithTimeZone", nullable)]
    pub expires_at: Option<DateTimeUtc>,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::UserId",
        to = "super::users::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
