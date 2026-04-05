use super::UserContext;
use axumkit_entity::common::Role;
use axumkit_errors::errors::Errors;

fn make_context(roles: Vec<Role>, is_banned: bool, is_authenticated: bool) -> UserContext {
    UserContext {
        roles,
        is_banned,
        is_authenticated,
    }
}

#[test]
fn test_user_context_has_role() {
    let ctx = make_context(vec![Role::Mod], false, true);
    assert!(ctx.has_role(Role::Mod));
    assert!(!ctx.has_role(Role::Admin));
}

#[test]
fn test_admin_has_all_capabilities() {
    let ctx = make_context(vec![Role::Admin], false, true);
    assert!(ctx.is_admin());
    assert!(ctx.require_role(Role::Mod).is_ok());
    assert!(ctx.require_role(Role::Admin).is_ok());
}

#[test]
fn test_admin_capabilities_do_not_change_direct_role_membership() {
    let ctx = make_context(vec![Role::Admin], false, true);
    // Admin can pass any require_role check
    assert!(ctx.require_role(Role::Mod).is_ok());
    // But has_role still reports actual membership
    assert!(!ctx.has_role(Role::Mod));
}

#[test]
fn test_anonymous_has_no_role() {
    let ctx = make_context(vec![], false, false);
    assert!(!ctx.is_admin());
    assert!(!ctx.has_role(Role::Mod));
    assert!(matches!(
        ctx.require_role(Role::Mod),
        Err(Errors::UserPermissionInsufficient)
    ));
}

#[test]
fn test_banned_user_denied() {
    let ctx = make_context(vec![Role::Mod], true, true);
    assert!(matches!(
        ctx.require_role(Role::Mod),
        Err(Errors::UserBanned)
    ));
}

#[test]
fn test_banned_admin_denied() {
    let ctx = make_context(vec![Role::Admin], true, true);
    assert!(matches!(
        ctx.require_role(Role::Admin),
        Err(Errors::UserBanned)
    ));
}

#[test]
fn test_mod_cannot_require_admin() {
    let ctx = make_context(vec![Role::Mod], false, true);
    assert!(matches!(
        ctx.require_role(Role::Admin),
        Err(Errors::UserPermissionInsufficient)
    ));
}

#[test]
fn test_require_not_banned_passes_for_unbanned() {
    let ctx = make_context(vec![], false, true);
    assert!(ctx.require_not_banned().is_ok());
}

#[test]
fn test_require_not_banned_fails_for_banned() {
    let ctx = make_context(vec![], true, true);
    assert!(matches!(
        ctx.require_not_banned(),
        Err(Errors::UserBanned)
    ));
}
