use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "actor_kind")]
pub enum ActorKind {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "anonymous")]
    Anonymous,
    #[sea_orm(string_value = "system")]
    System,
}
