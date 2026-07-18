use super::id_token::verify_google_id_token;
use crate::service::oauth::resolve_sign_in::resolve_oauth_sign_in;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::get_json_and_delete;
use dto::oauth::internal::SignInResult;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use tracing::debug;

/// Handle Google One Tap server-side sign-in.
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

    // Verify the single-use nonce: it must match the one we issued for this browser and
    // is consumed here (GETDEL), so a captured ID token cannot be replayed.
    let nonce = claims.nonce.ok_or_else(|| {
        debug!("Google One Tap ID token missing nonce claim");
        Errors::GoogleOneTapNonceInvalid
    })?;
    // Stored under the hashed nonce (hash-at-rest); hash the token's raw nonce
    // claim to derive the lookup key.
    let nonce_key = constants::oauth_one_tap_nonce_key(&hash_token(&nonce));
    let bound_anonymous_user_id: String = get_json_and_delete(
        redis_conn,
        &nonce_key,
        || Errors::GoogleOneTapNonceInvalid,
        |_| Errors::GoogleOneTapNonceInvalid,
    )
    .await?;
    if bound_anonymous_user_id != anonymous_user_id {
        debug!("Google One Tap nonce bound to a different anonymous id");
        return Err(Errors::GoogleOneTapNonceInvalid);
    }

    // Resolve common sign-in flow (existing user → session, new user → pending signup)
    resolve_oauth_sign_in(
        conn,
        redis_conn,
        OAuthProvider::Google,
        &claims.sub,
        claims.email,
        claims.picture,
        Some(anonymous_user_id),
        user_agent,
        ip_address,
    )
    .await
}
