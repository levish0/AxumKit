use super::{GoogleProvider, fetch_google_user_info};
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::find_oauth_connection::repository_find_oauth_connection;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::redis_cache::get_json_and_delete;
use constants::oauth_state_key;
use dto::oauth::request::OAuthAuthorizeFlow;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use uuid::Uuid;

/// Links Google OAuth to an existing account.
pub async fn service_link_google_oauth(
    conn: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    user_id: Uuid,
    code: &str,
    state: &str,
    anonymous_user_id: &str,
) -> ServiceResult<()> {
    // 1. Validate state and retrieve PKCE verifier from Redis (single-use via get_del)
    let state_key = oauth_state_key(state);
    let state_data: OAuthStateData = get_json_and_delete(
        redis_conn,
        &state_key,
        || Errors::OauthInvalidState,
        |_| Errors::OauthInvalidState,
    )
    .await?;

    // 2. Exchange authorization code for access token
    if state_data.provider != OAuthProvider::Google
        || state_data.flow != OAuthAuthorizeFlow::Link
        || state_data.anonymous_user_id != anonymous_user_id
    {
        return Err(Errors::OauthInvalidState);
    }

    let access_token =
        exchange_code::<GoogleProvider>(http_client, code, &state_data.pkce_verifier).await?;

    // 3. Fetch user info with access token
    let user_info = fetch_google_user_info(http_client, &access_token).await?;

    // 3-1. Check email verification status
    if !user_info.verified_email {
        return Err(Errors::OauthEmailNotVerified);
    }

    let txn = conn.begin().await?;

    // 4. Check if already linked to another account
    if repository_find_user_by_oauth(&txn, OAuthProvider::Google, &user_info.id)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 5. Check if Google is already linked to the current user
    if repository_find_oauth_connection(&txn, user_id, OAuthProvider::Google)
        .await?
        .is_some()
    {
        return Err(Errors::OauthAccountAlreadyLinked);
    }

    // 6. Create OAuth connection
    repository_create_oauth_connection(&txn, &user_id, OAuthProvider::Google, &user_info.id)
        .await?;

    txn.commit().await?;

    Ok(())
}
