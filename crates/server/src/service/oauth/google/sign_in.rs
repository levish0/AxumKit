use super::{GoogleProvider, fetch_google_user_info};
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::resolve_sign_in::resolve_oauth_sign_in;
use crate::service::oauth::types::OAuthStateData;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::get_json_and_delete;
use constants::oauth_state_key;
use dto::oauth::internal::SignInResult;
use dto::oauth::request::OAuthAuthorizeFlow;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Handles Google OAuth sign-in.
///
/// - Existing user: creates a session and returns Success
/// - New user: returns PendingSignup (requires complete-signup to finish registration)
pub async fn service_google_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    code: &str,
    state: &str,
    anonymous_user_id: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    // 1. Validate state and retrieve PKCE verifier from Redis (single-use via get_del)
    // Stored under the hashed state (hash-at-rest); hash the callback's raw
    // state to derive the lookup key.
    let state_key = oauth_state_key(&hash_token(state));
    let state_data: OAuthStateData = get_json_and_delete(
        redis_conn,
        &state_key,
        || Errors::OauthInvalidState,
        |_| Errors::OauthInvalidState,
    )
    .await?;

    // 2. Exchange authorization code for access token
    if state_data.provider != OAuthProvider::Google
        || state_data.flow != OAuthAuthorizeFlow::Login
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

    // 4. Resolve common sign-in flow (existing user → session, new user → pending signup)
    resolve_oauth_sign_in(
        conn,
        redis_conn,
        OAuthProvider::Google,
        &user_info.id,
        user_info.email,
        Some(user_info.picture),
        Some(anonymous_user_id),
        user_agent,
        ip_address,
    )
    .await
}
