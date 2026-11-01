use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum PushDeliveryState {
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "sent")]
    Sent,
    #[sea_orm(string_value = "skipped")]
    Skipped,
}
