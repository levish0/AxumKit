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
    #[serde(rename = "acl:group_create")]
    AclGroupCreate,
    #[serde(rename = "acl:group_delete")]
    AclGroupDelete,
    #[serde(rename = "acl:group_member_add")]
    AclGroupMemberAdd,
    #[serde(rename = "acl:group_member_remove")]
    AclGroupMemberRemove,
    #[serde(rename = "acl:group_permissions_replace")]
    AclGroupPermissionsReplace,
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
            ModerationAction::AclGroupCreate => "acl:group_create",
            ModerationAction::AclGroupDelete => "acl:group_delete",
            ModerationAction::AclGroupMemberAdd => "acl:group_member_add",
            ModerationAction::AclGroupMemberRemove => "acl:group_member_remove",
            ModerationAction::AclGroupPermissionsReplace => "acl:group_permissions_replace",
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
            "acl:group_create" => Ok(ModerationAction::AclGroupCreate),
            "acl:group_delete" => Ok(ModerationAction::AclGroupDelete),
            "acl:group_member_add" => Ok(ModerationAction::AclGroupMemberAdd),
            "acl:group_member_remove" => Ok(ModerationAction::AclGroupMemberRemove),
            "acl:group_permissions_replace" => Ok(ModerationAction::AclGroupPermissionsReplace),
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
