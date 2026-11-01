use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub event: String,
    pub request_id: Uuid,
    pub resource_id: Option<Uuid>,
    #[sea_orm(column_type = "String(StringLen::N(32))")]
    pub resource_type: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
