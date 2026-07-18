use crate::permission::UserContext;
use crate::permission::rule::Rule;
use constants::Permission;
use errors::errors::Errors;

/// Board-level permission facts.
#[derive(Debug, Clone)]
pub struct BoardFacts {
    pub is_disabled: bool,
}

/// Authorization rules for the board domain.
///
/// Reading is public; writing requires a signed-in, unbanned account; content
/// edits are owner-only; sanctions and board management are gated on RBAC
/// permissions (`board:moderate`, `board:pin_post`, `board:lock_post`,
/// `board:manage` — see `constants::Permission`).
pub enum BoardPermission {
    /// Can the actor see this board?
    View(BoardFacts),
    /// Can the actor create a new post or reply?
    Write(BoardFacts),
    /// Can the actor edit an existing post or comment?
    /// Owner-only: the author may edit while they still satisfy the board's
    /// write bar. Moderators sanction content (hide/delete/lock) but never
    /// rewrite someone else's words.
    EditContent { is_owner: bool, facts: BoardFacts },
    /// Can the actor delete an existing post or comment?
    /// The author may delete their own (unless banned); moderators may delete anything.
    DeleteContent { is_owner: bool },
    /// Can the actor moderate posts/comments (delete/hide others' content)?
    Moderate,
    /// Can the actor pin/unpin/reorder pinned posts?
    PinPost,
    /// Can the actor lock/unlock a post's comment thread?
    LockPost,
    /// Can the actor manage boards themselves (create/update/delete boards)?
    ManageBoard,
}

impl Rule for BoardPermission {
    fn check(&self, ctx: &UserContext) -> Result<(), Errors> {
        match self {
            Self::ManageBoard => ctx.require_perm(Permission::BoardManage),

            Self::View(f) => {
                // Admins always retain access — no board can be made invisible to them.
                if ctx.is_admin() {
                    return Ok(());
                }
                if f.is_disabled {
                    return Err(Errors::BoardNotFound);
                }
                // Reads are public and ban-exempt: a ban gates participation, not reading.
                Ok(())
            }

            Self::Write(f) => {
                // Escape hatch: admins can always post, so a board can never be
                // misconfigured into a state where nobody can write to it.
                if ctx.is_admin() {
                    return Ok(());
                }
                if f.is_disabled {
                    return Err(Errors::BoardNotFound);
                }
                if !ctx.is_authenticated {
                    return Err(Errors::UserUnauthorized);
                }
                ctx.require_not_banned()
            }

            // Editing is held to the same bar as writing: an author who can no longer
            // post to the board (lost access, banned) can no longer edit either.
            Self::EditContent { is_owner, facts } => {
                if *is_owner {
                    Self::Write(facts.clone()).check(ctx)
                } else {
                    // Content edits are owner-only. Moderators keep their
                    // sanction tools (hide/delete/lock via Moderate), but
                    // rewriting another user's words — silently, under the
                    // author's name — is not one of them.
                    Err(Errors::UserPermissionInsufficient)
                }
            }

            // Deleting one's own content only requires not being banned — board access
            // is irrelevant to retracting what you already published.
            Self::DeleteContent { is_owner } => {
                if *is_owner {
                    ctx.require_not_banned()
                } else {
                    Self::Moderate.check(ctx)
                }
            }

            Self::Moderate => ctx.require_perm(Permission::BoardModerate),
            Self::PinPost => ctx.require_perm(Permission::BoardPinPost),
            Self::LockPost => ctx.require_perm(Permission::BoardLockPost),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BoardFacts, BoardPermission};
    use crate::permission::UserContext;
    use crate::permission::rule::Rule;
    use constants::Permission;
    use entity::common::Role;
    use errors::errors::Errors;

    fn ctx(
        roles: Vec<Role>,
        permissions: &[Permission],
        banned: bool,
        authed: bool,
    ) -> UserContext {
        UserContext {
            roles,
            permissions: permissions.iter().copied().collect(),
            is_banned: banned,
            is_authenticated: authed,
        }
    }

    fn facts(disabled: bool) -> BoardFacts {
        BoardFacts {
            is_disabled: disabled,
        }
    }

    #[test]
    fn anonymous_can_view_but_not_write() {
        let anon = ctx(vec![], &[], false, false);
        assert!(BoardPermission::View(facts(false)).check(&anon).is_ok());
        assert!(matches!(
            BoardPermission::Write(facts(false)).check(&anon),
            Err(Errors::UserUnauthorized)
        ));
    }

    #[test]
    fn disabled_board_masks_as_not_found_except_for_admins() {
        let user = ctx(vec![], &[], false, true);
        assert!(matches!(
            BoardPermission::View(facts(true)).check(&user),
            Err(Errors::BoardNotFound)
        ));
        let admin = ctx(vec![Role::Admin], &[], false, true);
        assert!(BoardPermission::View(facts(true)).check(&admin).is_ok());
        assert!(BoardPermission::Write(facts(true)).check(&admin).is_ok());
    }

    #[test]
    fn banned_user_can_read_and_delete_nothing_but_own_is_blocked_too() {
        let banned = ctx(vec![], &[], true, true);
        assert!(BoardPermission::View(facts(false)).check(&banned).is_ok());
        assert!(matches!(
            BoardPermission::Write(facts(false)).check(&banned),
            Err(Errors::UserBanned)
        ));
        assert!(matches!(
            BoardPermission::DeleteContent { is_owner: true }.check(&banned),
            Err(Errors::UserBanned)
        ));
    }

    #[test]
    fn edit_is_owner_only() {
        let user = ctx(vec![], &[], false, true);
        assert!(
            BoardPermission::EditContent {
                is_owner: true,
                facts: facts(false)
            }
            .check(&user)
            .is_ok()
        );
        // Even a moderator cannot edit someone else's content.
        let moderator = ctx(vec![Role::Mod], &[], false, true);
        assert!(matches!(
            BoardPermission::EditContent {
                is_owner: false,
                facts: facts(false)
            }
            .check(&moderator),
            Err(Errors::UserPermissionInsufficient)
        ));
    }

    #[test]
    fn sanctions_ride_rbac_permissions() {
        let user = ctx(vec![], &[], false, true);
        assert!(BoardPermission::Moderate.check(&user).is_err());
        assert!(BoardPermission::PinPost.check(&user).is_err());

        // Mod holds the default set (moderate/pin/lock) but not manage.
        let moderator = ctx(vec![Role::Mod], &[], false, true);
        assert!(BoardPermission::Moderate.check(&moderator).is_ok());
        assert!(BoardPermission::PinPost.check(&moderator).is_ok());
        assert!(BoardPermission::LockPost.check(&moderator).is_ok());
        assert!(BoardPermission::ManageBoard.check(&moderator).is_err());

        // A plain user granted board:pin_post through a group can pin, nothing else.
        let granted = ctx(vec![], &[Permission::BoardPinPost], false, true);
        assert!(BoardPermission::PinPost.check(&granted).is_ok());
        assert!(BoardPermission::Moderate.check(&granted).is_err());
    }
}
