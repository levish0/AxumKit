use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

/// Declares the [`Permission`] enum and every representation derived from it.
///
/// One entry per permission keeps the enum variant, the serde rename, the
/// canonical `as_str` form, the `ALL` catalog (served by `GET /v0/permissions`),
/// and the `FromStr` parser in lockstep — a new permission cannot compile into
/// checks while silently missing from the admin catalog, which is exactly the
/// drift four hand-maintained parallel lists would allow.
macro_rules! define_permissions {
    ($($(#[$meta:meta])* $variant:ident => $code:literal),+ $(,)?) => {
        /// Grantable permission codename for `group_permissions.permission`.
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
            $(
                $(#[$meta])*
                #[serde(rename = $code)]
                $variant,
            )+
        }

        impl Permission {
            /// Returns the canonical string representation.
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Permission::$variant => $code,)+
                }
            }

            /// Every defined permission — the catalog served by `GET /v0/permissions`.
            pub const ALL: &'static [Permission] = &[
                $(Permission::$variant,)+
            ];
        }

        impl FromStr for Permission {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($code => Ok(Permission::$variant),)+
                    _ => Err(format!("Unknown permission: {}", s)),
                }
            }
        }
    };
}

define_permissions! {
    // Board (demo domain of the template)
    /// Pin/unpin/reorder pinned posts on a board.
    BoardPinPost => "board:pin_post",
    /// Lock/unlock a post (freeze its comment thread).
    BoardLockPost => "board:lock_post",
    /// Moderate others' content on a board (delete/hide posts and comments).
    BoardModerate => "board:moderate",
    /// Create/update/delete boards themselves.
    BoardManage => "board:manage",
}

impl Permission {
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

/// Convert Permission to String for DB storage.
pub fn permission_to_string(permission: Permission) -> String {
    permission.as_str().to_string()
}

/// Convert String from DB to Permission.
pub fn string_to_permission(s: &str) -> Option<Permission> {
    s.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::Permission;

    /// Every representation the macro derives must round-trip: catalog entry →
    /// codename → parsed variant → serde form.
    #[test]
    fn representations_stay_in_lockstep() {
        for permission in Permission::ALL {
            let code = permission.as_str();
            assert_eq!(code.parse::<Permission>().as_ref(), Ok(permission));
            let json = serde_json::to_string(permission).unwrap();
            assert_eq!(json, format!("\"{code}\""));
            assert_eq!(
                serde_json::from_str::<Permission>(&json).unwrap(),
                *permission
            );
        }
    }

    #[test]
    fn mod_defaults_are_defined_permissions() {
        for permission in Permission::MOD_DEFAULTS {
            assert!(Permission::ALL.contains(permission));
        }
    }
}
