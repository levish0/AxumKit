use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Grantable permission codename for `acl_group_permissions.permission`.
///
/// Django-style RBAC: a permission names an elevated capability; users hold it
/// through group membership (or implicitly through the `Mod`/`Admin` roles).
/// Baseline actions (reading, posting under the ban gate) are plain app logic
/// and deliberately NOT permissions.
///
/// Stored as TEXT (not a Postgres enum): the permission set grows with
/// features, so additions must not require an enum migration — the same
/// trade-off as `ModerationAction`. A stored codename that no longer parses
/// never matches any check (logged, fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Hash)]
pub enum Permission {
    // Board (demo domain of the template)
    /// Pin/unpin/reorder pinned posts on a board.
    #[serde(rename = "board:pin_post")]
    BoardPinPost,
    /// Lock/unlock a post (freeze its comment thread).
    #[serde(rename = "board:lock_post")]
    BoardLockPost,
    /// Moderate others' content on a board (delete/hide posts and comments).
    #[serde(rename = "board:moderate")]
    BoardModerate,
    /// Create/update/delete boards themselves.
    #[serde(rename = "board:manage")]
    BoardManage,
}

impl Permission {
    /// Returns the canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::BoardPinPost => "board:pin_post",
            Permission::BoardLockPost => "board:lock_post",
            Permission::BoardModerate => "board:moderate",
            Permission::BoardManage => "board:manage",
        }
    }

    pub const ALL: &'static [Permission] = &[
        Permission::BoardPinPost,
        Permission::BoardLockPost,
        Permission::BoardModerate,
        Permission::BoardManage,
    ];

    /// Permissions the `Mod` role holds implicitly (no group membership
    /// needed). `Admin` passes every check outright.
    pub const MOD_DEFAULTS: &'static [Permission] = &[
        Permission::BoardPinPost,
        Permission::BoardLockPost,
        Permission::BoardModerate,
    ];
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Permission {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "board:pin_post" => Ok(Permission::BoardPinPost),
            "board:lock_post" => Ok(Permission::BoardLockPost),
            "board:moderate" => Ok(Permission::BoardModerate),
            "board:manage" => Ok(Permission::BoardManage),
            _ => Err(format!("Unknown permission: {}", s)),
        }
    }
}

/// Convert Permission to String for DB storage.
pub fn permission_to_string(permission: Permission) -> String {
    permission.as_str().to_string()
}

/// Convert String from DB to Permission.
pub fn string_to_permission(s: &str) -> Option<Permission> {
    s.parse().ok()
}
