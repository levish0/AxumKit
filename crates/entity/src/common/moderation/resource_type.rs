use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "moderation_resource_type"
)]
pub enum ModerationResourceType {
    #[sea_orm(string_value = "user")]
    User,
    #[sea_orm(string_value = "system")]
    System,
    /// ACL group (group/member/permission management)
    #[sea_orm(string_value = "acl_group")]
    AclGroup,
    /// Board (board management actions)
    #[sea_orm(string_value = "board")]
    Board,
    /// Board post (pin/lock moderation)
    #[sea_orm(string_value = "board_post")]
    BoardPost,
}
