use super::{fetch_github_user_emails, verify_github_token};
use crate::service::oauth::resolve_sign_in::resolve_oauth_sign_in;
use dto::oauth::internal::SignInResult;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Native-app GitHub sign-in via a provider token (allauth `provider/token` pattern).
///
/// GitHub issues no ID token, so the app submits the OAuth **access token** it obtained through
/// its own in-app authorization. Since a bare access token carries no verifiable audience, it is
/// validated against GitHub's token introspection endpoint ([`verify_github_token`]), which
/// confirms it was issued for our OAuth app — restoring the audience binding the redirect/state
/// flow otherwise provides. The verified primary email is then derived from `/user/emails` (the
/// public profile email is never trusted), exactly like the browser flow. No browser-cookie
/// binding: new users get a pending-signup token bound only by its own secrecy.
pub async fn service_github_token_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    access_token: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    // Audience-bind the token to our app before trusting any identity it yields.
    let user_info = verify_github_token(http_client, access_token).await?;

    // Always derive the identity email from /user/emails and accept only a primary+verified
    // address — the public profile email is not guaranteed verified (matches the browser flow).
    let emails = fetch_github_user_emails(http_client, access_token).await?;
    let email = emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email)
        .ok_or(Errors::OauthUserInfoParseFailed(
            "No verified primary email found in GitHub account".to_string(),
        ))?;

    resolve_oauth_sign_in(
        conn,
        redis_conn,
        OAuthProvider::Github,
        &user_info.id.to_string(),
        email,
        Some(user_info.avatar_url),
        // Native app: no browser cookie jar → no anonymous-context binding on the pending token.
        None,
        user_agent,
        ip_address,
    )
    .await
}
