use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_flows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub state_hash: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub binding_hash: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub provider: String,
    #[sea_orm(column_type = "Text")]
    pub secret: String,
    pub account_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub security_version: Option<i64>,
    pub expires_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
