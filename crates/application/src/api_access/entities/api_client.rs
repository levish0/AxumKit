use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "api_clients")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(100))")]
    pub name: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub scopes: Json,
    pub active: bool,
    pub minute_limit: i64,
    pub month_limit: i64,
    pub minute_started_at: Option<DateTimeUtc>,
    pub month_started_at: Option<DateTimeUtc>,
    pub minute_used: i64,
    pub month_used: i64,
    pub created_at: DateTimeUtc,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
