use super::id_token::verify_google_id_token;
use crate::service::oauth::resolve_sign_in::resolve_oauth_sign_in;
use dto::oauth::internal::SignInResult;
use entity::common::OAuthProvider;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Native-app Google sign-in via a provider token (allauth `provider/token` pattern).
///
/// The app obtains the Google ID token itself through the native Google SDK (which runs its own
/// PKCE/redirect), then submits it here. There is no server-initiated redirect, state, or browser
/// cookie to bind — the ID token's `aud` (= our client id) + signature + expiry are the protection
/// (see [`verify_google_id_token`]). New users get a pending-signup token bound only by its own
/// secrecy (no anonymous-context binding); existing users get a session immediately.
pub async fn service_google_token_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    id_token: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    let claims = verify_google_id_token(http_client, id_token).await?;

    resolve_oauth_sign_in(
        conn,
        redis_conn,
        OAuthProvider::Google,
        &claims.sub,
        claims.email,
        claims.picture,
        // Native app: no browser cookie jar → no anonymous-context binding on the pending token.
        None,
        user_agent,
        ip_address,
    )
    .await
}
