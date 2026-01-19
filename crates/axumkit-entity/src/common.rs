use sea_orm::{DeriveActiveEnum, EnumIter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// OAuth 제공자: 소셜 로그인 제공 업체
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
    #[sea_orm(string_value = "microsoft")]
    Microsoft,
}

/// Action Log 리소스 타입: 사용자 활동 로그의 대상 리소스 종류
#[derive(
    Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Deserialize, Serialize, ToSchema,
)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "action_resource_type"
)]
pub enum ActionResourceType {
    /// 사용자
    #[sea_orm(string_value = "user")]
    User,
    /// 포스트
    #[sea_orm(string_value = "post")]
    Post,
}
