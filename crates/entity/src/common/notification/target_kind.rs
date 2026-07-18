use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "notification_target_kind"
)]
pub enum NotificationTargetKind {
    #[sea_orm(string_value = "none")]
    None,
    #[sea_orm(string_value = "board_post")]
    BoardPost,
    #[sea_orm(string_value = "board_comment")]
    BoardComment,
}
