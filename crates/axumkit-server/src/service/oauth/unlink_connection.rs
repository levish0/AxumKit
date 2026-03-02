use crate::repository::oauth::delete_oauth_connection::repository_delete_oauth_connection;
use crate::repository::oauth::list_oauth_connections::repository_list_oauth_connections_by_user_id;
use crate::repository::user::get_by_id::repository_get_user_by_id_for_update;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

/// Unlink an OAuth provider from a user account.
///
/// Safety rule:
/// - If the account has no password and only one OAuth connection left, unlink is denied.
pub async fn service_unlink_oauth(
    conn: &DatabaseConnection,
    user_id: Uuid,
    provider: OAuthProvider,
) -> ServiceResult<()> {
    let txn = conn.begin().await?;

    // Serialize per-user unlink flow to prevent concurrent last-factor removal.
    let user = repository_get_user_by_id_for_update(&txn, user_id).await?;

    let connections = repository_list_oauth_connections_by_user_id(&txn, user_id).await?;
    let oauth_count = connections.len();
    let has_target_provider = connections.iter().any(|c| c.provider == provider);

    if !has_target_provider {
        return Err(Errors::OauthConnectionNotFound);
    }

    if oauth_count == 1 && user.password.is_none() {
        return Err(Errors::OauthCannotUnlinkLastConnection);
    }

    repository_delete_oauth_connection(&txn, user_id, provider.clone()).await?;

    txn.commit().await?;

    info!(user_id = %user_id, provider = ?provider, "OAuth connection unlinked");

    Ok(())
}
