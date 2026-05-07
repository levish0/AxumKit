use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ModerationAction {
    #[serde(rename = "user:ban")]
    UserBan,
    #[serde(rename = "user:unban")]
    UserUnban,
    #[serde(rename = "user:grant_role")]
    UserGrantRole,
    #[serde(rename = "user:revoke_role")]
    UserRevokeRole,
    #[serde(rename = "search:reindex")]
    SearchReindex,
}

impl ModerationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationAction::UserBan => "user:ban",
            ModerationAction::UserUnban => "user:unban",
            ModerationAction::UserGrantRole => "user:grant_role",
            ModerationAction::UserRevokeRole => "user:revoke_role",
            ModerationAction::SearchReindex => "search:reindex",
        }
    }
}

impl fmt::Display for ModerationAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ModerationAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user:ban" => Ok(ModerationAction::UserBan),
            "user:unban" => Ok(ModerationAction::UserUnban),
            "user:grant_role" => Ok(ModerationAction::UserGrantRole),
            "user:revoke_role" => Ok(ModerationAction::UserRevokeRole),
            "search:reindex" => Ok(ModerationAction::SearchReindex),
            _ => Err(format!("Unknown moderation action: {}", s)),
        }
    }
}

pub fn moderation_action_to_string(action: ModerationAction) -> String {
    action.as_str().to_string()
}

pub fn string_to_moderation_action(s: &str) -> Option<ModerationAction> {
    s.parse().ok()
}
