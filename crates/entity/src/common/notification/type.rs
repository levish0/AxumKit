use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Notification type: the notification's top-level category
#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "notification_type")]
pub enum NotificationType {
    /// Board-related notifications
    #[sea_orm(string_value = "board")]
    Board,
    /// User-related notifications
    #[sea_orm(string_value = "user")]
    User,
    /// System notifications
    #[sea_orm(string_value = "system")]
    System,
}
