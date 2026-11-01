use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(254))")]
    pub email: String,
    #[sea_orm(column_type = "Text")]
    pub password_hash: Option<String>,
    pub verified: bool,
    pub active: bool,
    pub administrator: bool,
    pub security_version: i64,
    #[sea_orm(column_type = "Text")]
    pub totp_secret: Option<String>,
    pub totp_last_step: i64,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
