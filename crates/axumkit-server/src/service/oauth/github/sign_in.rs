use super::{GithubProvider, fetch_github_user_emails, fetch_github_user_info};
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::oauth::provider::client::exchange_code;
use crate::service::oauth::types::{OAuthStateData, PendingSignupData};
use crate::utils::redis_cache::{get_json_and_delete, issue_token_and_store_json_with_ttl};
use axumkit_config::ServerConfig;
use axumkit_constants::{oauth_pending_key, oauth_state_key};
use axumkit_dto::oauth::internal::SignInResult;
use axumkit_dto::oauth::request::OAuthAuthorizeFlow;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Handles GitHub OAuth sign-in.
///
/// - Existing user: creates a session and returns Success
/// - New user: returns PendingSignup (requires complete-signup to finish registration)
pub async fn service_github_sign_in<C>(
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
    let state_key = oauth_state_key(state);
    let state_data: OAuthStateData = get_json_and_delete(
        redis_conn,
        &state_key,
        || Errors::OauthInvalidState,
        |_| Errors::OauthInvalidState,
    )
    .await?;

    // 2. Exchange authorization code for access token
    if state_data.provider != OAuthProvider::Github
        || state_data.flow != OAuthAuthorizeFlow::Login
        || state_data.anonymous_user_id != anonymous_user_id
    {
        return Err(Errors::OauthInvalidState);
    }

    let access_token =
        exchange_code::<GithubProvider>(http_client, code, &state_data.pkce_verifier).await?;

    // 3. Fetch user info with access token
    let user_info = fetch_github_user_info(http_client, &access_token).await?;

    let email = if let Some(email) = user_info.email {
        email
    } else {
        let emails = fetch_github_user_emails(http_client, &access_token).await?;
        emails
            .into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email)
            .ok_or(Errors::OauthUserInfoParseFailed(
                "No verified primary email found in GitHub account".to_string(),
            ))?
    };

    // 4. Check for existing OAuth connection
    if let Some(existing_user) =
        repository_find_user_by_oauth(conn, OAuthProvider::Github, &user_info.id.to_string())
            .await?
    {
        // Existing user - create session and return Success
        let session = SessionService::create_session(
            redis_conn,
            existing_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        return Ok(SignInResult::Success(session.session_id));
    }

    // 5. New user - check for email duplication
    if repository_find_user_by_email(conn, email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 6. New user - store pending signup data in Redis
    let config = ServerConfig::get();
    let pending_data = PendingSignupData {
        provider: OAuthProvider::Github,
        provider_user_id: user_info.id.to_string(),
        anonymous_user_id: anonymous_user_id.to_string(),
        email: email.clone(),
        profile_image: Some(user_info.avatar_url),
    };

    let ttl_seconds = (config.oauth_pending_signup_ttl_minutes * 60) as u64;
    let pending_token = issue_token_and_store_json_with_ttl(
        redis_conn,
        || uuid::Uuid::new_v4().to_string(),
        oauth_pending_key,
        &pending_data,
        ttl_seconds,
    )
    .await?;

    Ok(SignInResult::PendingSignup {
        pending_token,
        email,
    })
}
