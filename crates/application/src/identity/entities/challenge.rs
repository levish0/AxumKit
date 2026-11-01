use crate::identity::ChallengePurpose;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "challenges")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub token_hash: String,
    pub account_id: Uuid,
    pub purpose: ChallengePurpose,
    pub security_version: i64,
    #[sea_orm(column_type = "Text")]
    pub secret: Option<String>,
    pub expires_at: DateTimeUtc,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
