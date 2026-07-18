use sea_orm::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(column_type = "Text", not_null, unique)]
    pub slug: String,
    #[sea_orm(column_type = "Text", not_null)]
    pub name: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    #[sea_orm(not_null, default_value = "0")]
    pub order: i32,
    #[sea_orm(not_null, default_value = "false")]
    pub is_disabled: bool,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub created_at: DateTimeUtc,
    #[sea_orm(column_type = "TimestampWithTimeZone", not_null)]
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::board_posts::Entity")]
    Posts,
}

impl Related<super::board_posts::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
