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

    // Board post
    #[serde(rename = "board:pin")]
    BoardPin,
    #[serde(rename = "board:unpin")]
    BoardUnpin,
    #[serde(rename = "board:lock")]
    BoardLock,
    #[serde(rename = "board:unlock")]
    BoardUnlock,
    #[serde(rename = "board:reorder_pins")]
    BoardReorderPins,

    // ACL
    #[serde(rename = "group:create")]
    GroupCreate,
    #[serde(rename = "group:delete")]
    GroupDelete,
    #[serde(rename = "group:member_add")]
    GroupMemberAdd,
    #[serde(rename = "group:member_remove")]
    GroupMemberRemove,
    #[serde(rename = "group:permissions_replace")]
    GroupPermissionsReplace,
}

impl ModerationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModerationAction::UserBan => "user:ban",
            ModerationAction::UserUnban => "user:unban",
            ModerationAction::UserGrantRole => "user:grant_role",
            ModerationAction::UserRevokeRole => "user:revoke_role",
            ModerationAction::SearchReindex => "search:reindex",
            ModerationAction::BoardPin => "board:pin",
            ModerationAction::BoardUnpin => "board:unpin",
            ModerationAction::BoardLock => "board:lock",
            ModerationAction::BoardUnlock => "board:unlock",
            ModerationAction::BoardReorderPins => "board:reorder_pins",
            ModerationAction::GroupCreate => "group:create",
            ModerationAction::GroupDelete => "group:delete",
            ModerationAction::GroupMemberAdd => "group:member_add",
            ModerationAction::GroupMemberRemove => "group:member_remove",
            ModerationAction::GroupPermissionsReplace => "group:permissions_replace",
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
            "board:pin" => Ok(ModerationAction::BoardPin),
            "board:unpin" => Ok(ModerationAction::BoardUnpin),
            "board:lock" => Ok(ModerationAction::BoardLock),
            "board:unlock" => Ok(ModerationAction::BoardUnlock),
            "board:reorder_pins" => Ok(ModerationAction::BoardReorderPins),
            "group:create" => Ok(ModerationAction::GroupCreate),
            "group:delete" => Ok(ModerationAction::GroupDelete),
            "group:member_add" => Ok(ModerationAction::GroupMemberAdd),
            "group:member_remove" => Ok(ModerationAction::GroupMemberRemove),
            "group:permissions_replace" => Ok(ModerationAction::GroupPermissionsReplace),
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
