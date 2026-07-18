use crate::repository::group_members::repository_find_active_group_memberships;
use crate::repository::group_permissions::repository_find_permissions_for_groups;
use crate::repository::user::repository_find_user_by_id;
use crate::repository::user::user_bans::repository_find_user_ban;
use crate::repository::user::user_roles::repository_find_user_roles;
use crate::service::auth::session_types::SessionContext;
use constants::Permission;
use entity::common::Role;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use std::collections::HashSet;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserContext {
    pub roles: Vec<Role>,
    /// Union of the permissions granted through the user's active group
    /// memberships (Django-style `user.get_group_permissions()`). Role-implied
    /// permissions are resolved in [`UserContext::has_perm`], not stored here.
    pub permissions: HashSet<Permission>,
    pub is_banned: bool,
    pub is_authenticated: bool,
}

impl UserContext {
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    pub fn require_role(&self, role: Role) -> Result<(), Errors> {
        self.require_not_banned()?;
        if !self.is_admin() && !self.has_role(role) {
            return Err(Errors::UserPermissionInsufficient);
        }
        Ok(())
    }

    /// Django's `user.has_perm`, resolved from three sources in order:
    /// Admin passes everything (anti-lockout — admins can never lock
    /// themselves out via group edits), `Mod` holds its built-in default set,
    /// and everyone else needs a grant through group membership. Banned users
    /// hold no permissions at all (ban hard gate).
    pub fn has_perm(&self, permission: Permission) -> bool {
        if self.is_banned {
            return false;
        }
        if self.is_admin() {
            return true;
        }
        if self.has_role(Role::Mod) && Permission::MOD_DEFAULTS.contains(&permission) {
            return true;
        }
        self.permissions.contains(&permission)
    }

    /// Check-or-deny form of [`UserContext::has_perm`]. The denial carries the
    /// permission codename so the client learns exactly which capability was
    /// missing (`permission:denied` + `board:pin_post`).
    pub fn require_perm(&self, permission: Permission) -> Result<(), Errors> {
        self.require_not_banned()?;
        if self.has_perm(permission) {
            Ok(())
        } else {
            Err(Errors::PermissionDenied(permission.as_str().to_string()))
        }
    }

    pub fn require_not_banned(&self) -> Result<(), Errors> {
        if self.is_banned {
            return Err(Errors::UserBanned);
        }
        Ok(())
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }
}

pub struct PermissionService;

impl PermissionService {
    pub async fn get_context<C>(
        conn: &C,
        session: Option<&SessionContext>,
    ) -> Result<UserContext, Errors>
    where
        C: ConnectionTrait,
    {
        let is_authenticated = session.is_some();

        let (roles, permissions, is_banned) = match session {
            Some(session) => {
                let roles = Self::fetch_roles(conn, session.user_id).await?;
                let permissions = Self::fetch_group_permissions(conn, session.user_id).await?;
                let is_banned = Self::fetch_ban(conn, session.user_id).await?;
                (roles, permissions, is_banned)
            }
            None => (vec![], HashSet::new(), false),
        };

        Ok(UserContext {
            roles,
            permissions,
            is_banned,
            is_authenticated,
        })
    }

    pub async fn require_role<C>(
        conn: &C,
        session: Option<&SessionContext>,
        role: Role,
    ) -> Result<UserContext, Errors>
    where
        C: ConnectionTrait,
    {
        let ctx = Self::get_context(conn, session).await?;
        ctx.require_role(role)?;
        Ok(ctx)
    }

    /// Convenience wrapper: load the context and demand one permission.
    pub async fn require_perm<C>(
        conn: &C,
        session: Option<&SessionContext>,
        permission: Permission,
    ) -> Result<UserContext, Errors>
    where
        C: ConnectionTrait,
    {
        let ctx = Self::get_context(conn, session).await?;
        ctx.require_perm(permission)?;
        Ok(ctx)
    }

    pub async fn require_admin_for_target<C>(
        conn: &C,
        session: Option<&SessionContext>,
        target_user_id: Uuid,
    ) -> Result<UserContext, Errors>
    where
        C: ConnectionTrait,
    {
        let ctx = Self::get_context(conn, session).await?;
        ctx.require_role(Role::Admin)?;

        if let Some(session) = session
            && session.user_id == target_user_id
        {
            return Err(Errors::CannotManageSelf);
        }

        repository_find_user_by_id(conn, target_user_id)
            .await?
            .ok_or(Errors::UserNotFound)?;

        let target_roles = Self::fetch_roles(conn, target_user_id).await?;
        if target_roles.contains(&Role::Admin) {
            return Err(Errors::CannotManageHigherOrEqualRole);
        }

        Ok(ctx)
    }

    async fn fetch_roles<C>(conn: &C, user_id: Uuid) -> Result<Vec<Role>, Errors>
    where
        C: ConnectionTrait,
    {
        repository_find_user_roles(conn, user_id).await
    }

    /// Loads the union of permissions granted through active group
    /// memberships. Stored codenames that no longer parse never match — the
    /// grant silently dies with the removed feature (logged, fail-closed).
    async fn fetch_group_permissions<C>(
        conn: &C,
        user_id: Uuid,
    ) -> Result<HashSet<Permission>, Errors>
    where
        C: ConnectionTrait,
    {
        let memberships =
            repository_find_active_group_memberships(conn, Some(user_id), None).await?;
        if memberships.is_empty() {
            return Ok(HashSet::new());
        }

        let group_ids: Vec<Uuid> = memberships
            .iter()
            .map(|(member, _)| member.group_id)
            .collect();

        let grants = repository_find_permissions_for_groups(conn, &group_ids).await?;

        let mut permissions = HashSet::new();
        for grant in grants {
            match grant.permission.parse::<Permission>() {
                Ok(permission) => {
                    permissions.insert(permission);
                }
                Err(_) => {
                    warn!(
                        group_id = %grant.group_id,
                        permission = %grant.permission,
                        "Stored group permission no longer parses; ignoring"
                    );
                }
            }
        }

        Ok(permissions)
    }

    async fn fetch_ban<C>(conn: &C, user_id: Uuid) -> Result<bool, Errors>
    where
        C: ConnectionTrait,
    {
        let ban = repository_find_user_ban(conn, user_id).await?;
        Ok(ban.is_some())
    }
}
