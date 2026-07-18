use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::store_json_for_token_with_ttl;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Issues a single-use Google One Tap nonce and stores it in Redis.
///
/// The nonce is bound to the caller's anonymous id and consumed (single-use) during
/// One Tap sign-in, preventing replay of a captured Google ID token. Mirrors the
/// OAuth state issuance flow (`generate_oauth_url`): random token → key-derived Redis
/// entry with TTL.
pub async fn service_issue_google_one_tap_nonce(
    redis_conn: &ConnectionManager,
    anonymous_user_id: &str,
) -> ServiceResult<String> {
    // Fully-random value: an anti-replay nonce favors unpredictability over ordering.
    let nonce = Uuid::new_v4().to_string();

    // Key by the hashed nonce (hash-at-rest, matching the OAuth state entry): a
    // Redis snapshot/leak must not expose live nonce values. The raw nonce only
    // travels to the client.
    store_json_for_token_with_ttl(
        redis_conn,
        &nonce,
        |token| constants::oauth_one_tap_nonce_key(&hash_token(token)),
        &anonymous_user_id.to_string(),
        constants::OAUTH_ONE_TAP_NONCE_TTL_SECONDS,
    )
    .await?;

    Ok(nonce)
}
