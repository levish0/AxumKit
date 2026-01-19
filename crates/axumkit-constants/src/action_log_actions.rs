use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Action Log Action enum (action_logs.action 필드에 저장됨)
/// 포맷: "{resource}:{operation}"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ActionLogAction {
    // ==================== Post Actions ====================
    /// 포스트 생성
    #[serde(rename = "post:create")]
    PostCreate,
    /// 포스트 편집
    #[serde(rename = "post:edit")]
    PostEdit,
    /// 포스트 삭제
    #[serde(rename = "post:delete")]
    PostDelete,

    // ==================== User Actions ====================
    /// 유저 생성
    #[serde(rename = "user:create")]
    UserCreate,
    /// 유저 프로필 편집
    #[serde(rename = "user:edit")]
    UserEdit,

    // ==================== Auth Actions ====================
    /// 로그인
    #[serde(rename = "auth:login")]
    AuthLogin,
    /// 로그아웃
    #[serde(rename = "auth:logout")]
    AuthLogout,
    /// OAuth 로그인
    #[serde(rename = "auth:oauth_login")]
    AuthOAuthLogin,

    // ==================== OAuth Actions ====================
    /// OAuth 연결
    #[serde(rename = "oauth:link")]
    OAuthLink,
    /// OAuth 연결 해제
    #[serde(rename = "oauth:unlink")]
    OAuthUnlink,
}

impl ActionLogAction {
    /// Convert to database string value
    pub fn as_str(&self) -> &'static str {
        match self {
            // Post
            ActionLogAction::PostCreate => "post:create",
            ActionLogAction::PostEdit => "post:edit",
            ActionLogAction::PostDelete => "post:delete",
            // User
            ActionLogAction::UserCreate => "user:create",
            ActionLogAction::UserEdit => "user:edit",
            // Auth
            ActionLogAction::AuthLogin => "auth:login",
            ActionLogAction::AuthLogout => "auth:logout",
            ActionLogAction::AuthOAuthLogin => "auth:oauth_login",
            // OAuth
            ActionLogAction::OAuthLink => "oauth:link",
            ActionLogAction::OAuthUnlink => "oauth:unlink",
        }
    }
}

impl fmt::Display for ActionLogAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ActionLogAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Post
            "post:create" => Ok(ActionLogAction::PostCreate),
            "post:edit" => Ok(ActionLogAction::PostEdit),
            "post:delete" => Ok(ActionLogAction::PostDelete),
            // User
            "user:create" => Ok(ActionLogAction::UserCreate),
            "user:edit" => Ok(ActionLogAction::UserEdit),
            // Auth
            "auth:login" => Ok(ActionLogAction::AuthLogin),
            "auth:logout" => Ok(ActionLogAction::AuthLogout),
            "auth:oauth_login" => Ok(ActionLogAction::AuthOAuthLogin),
            // OAuth
            "oauth:link" => Ok(ActionLogAction::OAuthLink),
            "oauth:unlink" => Ok(ActionLogAction::OAuthUnlink),
            _ => Err(format!("Unknown action log action: {}", s)),
        }
    }
}

/// Convert ActionLogAction to String for DB storage
pub fn action_log_action_to_string(action: ActionLogAction) -> String {
    action.as_str().to_string()
}

/// Convert String from DB to ActionLogAction
pub fn string_to_action_log_action(s: &str) -> Option<ActionLogAction> {
    s.parse().ok()
}
