use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
#[serde(rename_all = "snake_case")]
pub enum PushPlatform {
    #[sea_orm(string_value = "android")]
    Android,
    #[sea_orm(string_value = "ios")]
    Ios,
    #[sea_orm(string_value = "web")]
    Web,
}
