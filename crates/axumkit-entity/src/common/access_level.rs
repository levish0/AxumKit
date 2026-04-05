use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    EnumIter,
    DeriveActiveEnum,
    Deserialize,
    Serialize,
    ToSchema,
    Hash,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "access_level")]
pub enum AccessLevel {
    #[sea_orm(string_value = "everyone")]
    #[serde(rename = "everyone")]
    Everyone,
    #[sea_orm(string_value = "user")]
    #[serde(rename = "user")]
    User,
    #[sea_orm(string_value = "mod")]
    #[serde(rename = "mod")]
    Mod,
    #[sea_orm(string_value = "admin")]
    #[serde(rename = "admin")]
    Admin,
}
