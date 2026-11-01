use crate::notifications::PushPlatform;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "push_devices")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    pub session_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub token_hash: String,
    #[sea_orm(column_type = "Text")]
    pub encrypted_token: String,
    pub platform: PushPlatform,
    pub version: i64,
    pub expires_at: DateTimeUtc,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
