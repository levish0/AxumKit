use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::auth::verify_email::find_pending_email_signup_by_email;
use crate::service::oauth::types::{PendingSignupData, PendingSignupTokenState};
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use constants::oauth_pending_key;
use dto::oauth::internal::SignInResult;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Shared OAuth sign-in handling after provider authentication.
///
/// Each provider sign-in (authorization code / one-tap / native provider-token) only performs
/// provider-specific token verification and user-info extraction, then delegates the common flow
/// here:
/// - If a connected account exists, create a session and return `Success`.
/// - Otherwise reject email collisions (existing account or pending email/password signup) and
///   issue a pending-signup token, returning `PendingSignup`.
pub async fn resolve_oauth_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    provider: OAuthProvider,
    provider_user_id: &str,
    email: String,
    profile_image: Option<String>,
    // Completion binding stored on the pending token: `Some` for browser flows (redirect/One-Tap),
    // `None` for the native-app `provider/token` flow (bound by the pending token's secrecy alone).
    pending_anonymous_user_id: Option<&str>,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    // Existing OAuth connection → sign in immediately.
    if let Some(existing_user) =
        repository_find_user_by_oauth(conn, provider.clone(), provider_user_id).await?
    {
        let (raw_token, _session) = SessionService::create_session(
            redis_conn,
            existing_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        return Ok(SignInResult::Success(raw_token));
    }

    // New user: reject if the email already belongs to another account (prevent auto-linking).
    if repository_find_user_by_email(conn, email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // New user: reject if a pending email/password signup already holds this email.
    if find_pending_email_signup_by_email(redis_conn, &email)
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // New user: store pending-signup data in Redis (consumed by complete-signup after handle input).
    let config = ServerConfig::get();
    let pending_data = PendingSignupData {
        provider,
        provider_user_id: provider_user_id.to_string(),
        anonymous_user_id: pending_anonymous_user_id.map(str::to_string),
        email: email.clone(),
        profile_image,
    };

    let ttl_seconds = (config.oauth_pending_signup_ttl_minutes * 60) as u64;
    let pending_state = PendingSignupTokenState::Pending { data: pending_data };
    // Store under the token's hash so a Redis leak yields only non-replayable hashes; the raw
    // token lives only in the response body (parity with the session-token at-rest scheme).
    let pending_token = issue_token_and_store_json_with_ttl(
        redis_conn,
        || uuid::Uuid::new_v4().to_string(),
        |token| oauth_pending_key(&hash_token(token)),
        &pending_state,
        ttl_seconds,
    )
    .await?;

    Ok(SignInResult::PendingSignup {
        pending_token,
        email,
    })
}
