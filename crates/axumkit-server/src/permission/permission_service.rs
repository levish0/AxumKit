use crate::repository::user::repository_find_user_by_id;
use crate::repository::user::user_bans::repository_find_user_ban;
use crate::repository::user::user_roles::repository_find_user_roles;
use crate::service::auth::session_types::SessionContext;
use axumkit_entity::common::Role;
use axumkit_errors::errors::Errors;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserContext {
    pub roles: Vec<Role>,
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

        let (roles, is_banned) = match session {
            Some(session) => {
                let roles = Self::fetch_roles(conn, session.user_id).await?;
                let is_banned = Self::fetch_ban(conn, session.user_id).await?;
                (roles, is_banned)
            }
            None => (vec![], false),
        };

        Ok(UserContext {
            roles,
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
            return Err(Errors::CannotManageHigherOrEqualRole);
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

    async fn fetch_ban<C>(conn: &C, user_id: Uuid) -> Result<bool, Errors>
    where
        C: ConnectionTrait,
    {
        let ban = repository_find_user_ban(conn, user_id).await?;
        Ok(ban.is_some())
    }
}
