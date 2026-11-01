use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    #[sea_orm(string_value = "uploading")]
    Uploading,
    #[sea_orm(string_value = "transferring")]
    Transferring,
    #[sea_orm(string_value = "ready")]
    Ready,
    #[sea_orm(string_value = "deleting")]
    Deleting,
    #[sea_orm(string_value = "deleted")]
    Deleted,
}
