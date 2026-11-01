use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(80))")]
    pub display_name: String,
    #[sea_orm(column_type = "Text")]
    pub bio: String,
    pub version: i64,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub search_reconciled_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
