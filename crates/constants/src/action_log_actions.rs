use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Action Log Action enum (stored in action_logs.action field)
/// Format: "{resource}:{operation}"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ActionLogAction {
    // ==================== Post Actions ====================
    /// Post created
    #[serde(rename = "post:create")]
    PostCreate,
    /// Post edited
    #[serde(rename = "post:edit")]
    PostEdit,
    /// Post deleted
    #[serde(rename = "post:delete")]
    PostDelete,

    // ==================== User Actions ====================
    /// User created
    #[serde(rename = "user:create")]
    UserCreate,
    /// User profile edited
    #[serde(rename = "user:edit")]
    UserEdit,

    // ==================== Auth Actions ====================
    /// Login
    #[serde(rename = "auth:login")]
    AuthLogin,
    /// Logout
    #[serde(rename = "auth:logout")]
    AuthLogout,
    /// OAuth login
    #[serde(rename = "auth:oauth_login")]
    AuthOAuthLogin,

    // ==================== OAuth Actions ====================
    /// OAuth connected
    #[serde(rename = "oauth:link")]
    OAuthLink,
    /// OAuth disconnected
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
