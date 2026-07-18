//! User ban lookups backed by the `user_bans` table.
//!
//! A "ban" is an active row in that table; expiry is filtered at read time by
//! the repository layer.

use crate::repository::user::user_bans::{
    repository_find_active_bans_for_users, repository_find_user_ban,
};
use entity::user_bans::Model as UserBanModel;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use std::collections::HashMap;
use uuid::Uuid;

/// Finds the active ban for one user. Returns `None` when the user is not banned.
pub async fn find_active_user_ban<C>(
    conn: &C,
    user_id: Uuid,
) -> Result<Option<UserBanModel>, Errors>
where
    C: ConnectionTrait,
{
    repository_find_user_ban(conn, user_id).await
}

/// Finds active bans for a set of users, keyed by user id.
pub async fn find_active_user_bans_by_ids<C>(
    conn: &C,
    user_ids: &[Uuid],
) -> Result<HashMap<Uuid, UserBanModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(repository_find_active_bans_for_users(conn, user_ids)
        .await?
        .into_iter()
        .map(|ban| (ban.user_id, ban))
        .collect())
}
