use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "oauth_provider")]
pub enum OAuthProvider {
    /// Google OAuth
    #[sea_orm(string_value = "google")]
    Google,
    /// GitHub OAuth
    #[sea_orm(string_value = "github")]
    Github,
    /// Discord OAuth
    #[sea_orm(string_value = "discord")]
    Discord,
    /// X (Twitter) OAuth
    #[sea_orm(string_value = "x")]
    X,
    /// Microsoft OAuth
    #[sea_orm(string_value = "Microsoft")]
    Microsoft,
}
