use super::id_token::verify_google_id_token;
use crate::service::oauth::resolve_sign_in::resolve_oauth_sign_in;
use dto::oauth::internal::SignInResult;
use entity::common::OAuthProvider;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;

/// Handle Google One Tap server-side sign-in.
///
/// Verifies the One Tap credential (a Google ID token) via the shared [`verify_google_id_token`]
/// — JWKS signature, issuer, audience pinned to our client id, expiry, and verified email — then
/// delegates account resolution / pending-signup issuance to [`resolve_oauth_sign_in`]. The
/// browser context (`anonymous_user_id`) is carried as the pending-token completion binding.
pub async fn service_google_one_tap_sign_in<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &reqwest::Client,
    credential: &str,
    anonymous_user_id: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<SignInResult>
where
    C: ConnectionTrait,
{
    let claims = verify_google_id_token(http_client, credential).await?;

    resolve_oauth_sign_in(
        conn,
        redis_conn,
        OAuthProvider::Google,
        &claims.sub,
        claims.email,
        claims.picture,
        // Browser flow: bind the pending token to the same anonymous browser context.
        Some(anonymous_user_id),
        user_agent,
        ip_address,
    )
    .await
}
