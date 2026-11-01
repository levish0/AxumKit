use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "oauth_native_flows")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub flow_hash: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub nonce_hash: String,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub provider: String,
    pub expires_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
