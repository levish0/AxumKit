use crate::identity::MailDeliveryState;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mail_deliveries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub account_id: Uuid,
    #[sea_orm(column_type = "String(StringLen::N(64))")]
    pub challenge_hash: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub ciphertext: Option<String>,
    pub state: MailDeliveryState,
    pub expires_at: DateTimeUtc,
    pub accepted_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
