use super::UserContext;
use constants::Permission;
use entity::common::Role;
use errors::errors::Errors;
use std::collections::HashSet;

fn make_context(roles: Vec<Role>, is_banned: bool, is_authenticated: bool) -> UserContext {
    UserContext {
        roles,
        permissions: HashSet::new(),
        is_banned,
        is_authenticated,
    }
}

fn with_permissions(mut ctx: UserContext, permissions: &[Permission]) -> UserContext {
    ctx.permissions = permissions.iter().copied().collect();
    ctx
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
    assert!(matches!(ctx.require_not_banned(), Err(Errors::UserBanned)));
}

#[test]
fn test_admin_has_every_permission() {
    let ctx = make_context(vec![Role::Admin], false, true);
    for permission in Permission::ALL {
        assert!(ctx.has_perm(*permission));
    }
}

#[test]
fn test_mod_holds_default_permission_set_only() {
    let ctx = make_context(vec![Role::Mod], false, true);
    for permission in Permission::MOD_DEFAULTS {
        assert!(ctx.has_perm(*permission));
    }
    // board:manage is not in the Mod default set
    assert!(!ctx.has_perm(Permission::BoardManage));
}

#[test]
fn test_group_grant_confers_permission() {
    let ctx = with_permissions(
        make_context(vec![], false, true),
        &[Permission::BoardPinPost],
    );
    assert!(ctx.has_perm(Permission::BoardPinPost));
    assert!(!ctx.has_perm(Permission::BoardModerate));
}

#[test]
fn test_denial_carries_permission_codename() {
    let ctx = make_context(vec![], false, true);
    match ctx.require_perm(Permission::BoardPinPost) {
        Err(Errors::AclDenied(codename)) => assert_eq!(codename, "board:pin_post"),
        other => panic!("expected AclDenied, got {:?}", other),
    }
}

#[test]
fn test_ban_gates_all_permissions() {
    // Even a grant, the Mod default set, and the Admin bypass are all void
    // while banned — the ban hard gate runs first.
    let granted = with_permissions(
        make_context(vec![], true, true),
        &[Permission::BoardPinPost],
    );
    assert!(!granted.has_perm(Permission::BoardPinPost));

    let banned_admin = make_context(vec![Role::Admin], true, true);
    assert!(!banned_admin.has_perm(Permission::BoardManage));
    assert!(matches!(
        banned_admin.require_perm(Permission::BoardManage),
        Err(Errors::UserBanned)
    ));
}

#[test]
fn test_anonymous_has_no_permissions() {
    let ctx = make_context(vec![], false, false);
    for permission in Permission::ALL {
        assert!(!ctx.has_perm(*permission));
    }
}
