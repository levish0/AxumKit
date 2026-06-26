use config::ServerConfig;
use errors::errors::{Errors, ServiceResult};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::header::{CACHE_CONTROL, HeaderValue};
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::debug;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const DEFAULT_JWKS_CACHE_TTL_SECONDS: u64 = 300;
/// Minimum spacing between *forced* JWKS refreshes (the unknown-`kid` retry path). Bounds how often
/// attacker-controlled `kid`s can make us reach out to Google.
const FORCED_REFRESH_MIN_INTERVAL: Duration = Duration::from_secs(60);

static GOOGLE_JWKS_CACHE: LazyLock<RwLock<Option<CachedGoogleJwks>>> =
    LazyLock::new(|| RwLock::new(None));
static GOOGLE_JWKS_REFRESH_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
static GOOGLE_JWKS_FORCED_REFRESH_AT: LazyLock<Mutex<Option<Instant>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Debug, Clone)]
struct CachedGoogleJwks {
    jwks: JwkSet,
    expires_at: Instant,
}

/// Verified claims from a Google ID token.
#[derive(Debug, Deserialize)]
pub struct GoogleIdTokenClaims {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub picture: Option<String>,
    /// Single-use nonce echoed back by Google. Present in the One Tap web flow; absent/ignored in
    /// the native-app `provider/token` flow, where the token's `aud` (= our client id) + signature
    /// + expiry are the protection.
    pub nonce: Option<String>,
}

/// Verify a Google ID token: validate the JWT signature against Google's JWKS, enforce the
/// issuer/audience/expiry, and require a verified email. Returns the decoded claims.
///
/// This is the audience-bound core shared by both Google sign-in flows. The `aud` claim is pinned
/// to our client id, so a token Google minted for a different app is rejected — which is exactly
/// what lets the native-app flow skip the browser-cookie binding the redirect flow needs.
pub async fn verify_google_id_token(
    http_client: &reqwest::Client,
    credential: &str,
) -> ServiceResult<GoogleIdTokenClaims> {
    let header = decode_header(credential).map_err(|e| {
        debug!(error = %e, "Failed to decode Google ID token header");
        Errors::GoogleInvalidIdToken
    })?;
    let kid = header.kid.ok_or_else(|| {
        debug!("Google ID token header missing 'kid' field");
        Errors::GoogleInvalidIdToken
    })?;

    let (jwks, from_cache) = get_google_jwks(http_client, false).await?;
    let decoding_key = if let Some(jwk) = jwks.find(&kid) {
        DecodingKey::from_jwk(jwk).map_err(|e| {
            debug!(error = %e, "Failed to build decoding key from JWK");
            Errors::GoogleInvalidIdToken
        })?
    } else if from_cache {
        // Google can rotate signing keys before our cached JWKS expires.
        // Retry once with a forced refresh before classifying the token as invalid.
        let (refreshed_jwks, _) = get_google_jwks(http_client, true).await?;
        let jwk = refreshed_jwks.find(&kid).ok_or_else(|| {
            debug!(kid = %kid, "kid not found in refreshed JWKS");
            Errors::GoogleInvalidIdToken
        })?;
        DecodingKey::from_jwk(jwk).map_err(|e| {
            debug!(error = %e, "Failed to build decoding key from refreshed JWK");
            Errors::GoogleInvalidIdToken
        })?
    } else {
        debug!(kid = %kid, "kid not found in freshly fetched JWKS");
        return Err(Errors::GoogleInvalidIdToken);
    };

    let config = ServerConfig::get();
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.validate_nbf = true;
    validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
    validation.set_audience(&[config.google_client_id.as_str()]);

    let token_data = decode::<GoogleIdTokenClaims>(credential, &decoding_key, &validation)
        .map_err(|e| {
            debug!(error = %e, "Google ID token validation failed");
            Errors::GoogleInvalidIdToken
        })?;

    if !token_data.claims.email_verified {
        return Err(Errors::OauthEmailNotVerified);
    }

    Ok(token_data.claims)
}

async fn get_google_jwks(
    http_client: &reqwest::Client,
    force_refresh: bool,
) -> ServiceResult<(JwkSet, bool)> {
    if !force_refresh {
        let cache = GOOGLE_JWKS_CACHE.read().await;
        if let Some(cached) = cache.as_ref()
            && Instant::now() < cached.expires_at
        {
            return Ok((cached.jwks.clone(), true));
        }
    }

    let _refresh_guard = GOOGLE_JWKS_REFRESH_LOCK.lock().await;

    if force_refresh {
        // The forced path is reached only when a token's `kid` is absent from the cached JWKS, and
        // `kid` is attacker-controlled (`decode_header` needs no valid signature). Throttle forced
        // refreshes so a flood of bogus kids can't make us fetch from Google once per request
        // (which would get our egress rate-limited and break Google sign-in for everyone). Inside
        // the window, serve the current cache; the caller then rejects the unknown kid.
        let mut forced_at = GOOGLE_JWKS_FORCED_REFRESH_AT.lock().await;
        let within_window =
            matches!(*forced_at, Some(at) if Instant::now() < at + FORCED_REFRESH_MIN_INTERVAL);
        if within_window && let Some(cached) = GOOGLE_JWKS_CACHE.read().await.as_ref() {
            return Ok((cached.jwks.clone(), true));
        }
        *forced_at = Some(Instant::now());
    } else {
        // Double-check after acquiring the refresh lock so only one request fetches
        // JWKS when the cache is cold or expired.
        let cache = GOOGLE_JWKS_CACHE.read().await;
        if let Some(cached) = cache.as_ref()
            && Instant::now() < cached.expires_at
        {
            return Ok((cached.jwks.clone(), true));
        }
    }

    let (jwks, cache_ttl_seconds) = fetch_google_jwks(http_client).await?;

    {
        let mut cache = GOOGLE_JWKS_CACHE.write().await;
        *cache = Some(CachedGoogleJwks {
            jwks: jwks.clone(),
            expires_at: Instant::now() + Duration::from_secs(cache_ttl_seconds),
        });
    }

    Ok((jwks, false))
}

async fn fetch_google_jwks(http_client: &reqwest::Client) -> ServiceResult<(JwkSet, u64)> {
    let response = http_client
        .get(GOOGLE_JWKS_URL)
        .send()
        .await
        .map_err(|_| Errors::GoogleJwksFetchFailed)?;

    if !response.status().is_success() {
        return Err(Errors::GoogleJwksFetchFailed);
    }

    let cache_ttl_seconds = response
        .headers()
        .get(CACHE_CONTROL)
        .and_then(|value: &HeaderValue| value.to_str().ok())
        .and_then(parse_cache_control_max_age)
        .unwrap_or(DEFAULT_JWKS_CACHE_TTL_SECONDS);

    let jwks = response
        .json::<JwkSet>()
        .await
        .map_err(|_| Errors::GoogleJwksParseFailed)?;

    Ok((jwks, cache_ttl_seconds))
}

fn parse_cache_control_max_age(cache_control: &str) -> Option<u64> {
    cache_control.split(',').find_map(|directive| {
        directive
            .trim()
            .strip_prefix("max-age=")
            .and_then(|value| value.parse::<u64>().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::parse_cache_control_max_age;

    #[test]
    fn parses_max_age_from_cache_control() {
        assert_eq!(
            parse_cache_control_max_age("public, max-age=24131, must-revalidate"),
            Some(24131)
        );
    }

    #[test]
    fn returns_none_when_max_age_missing() {
        assert_eq!(parse_cache_control_max_age("public, must-revalidate"), None);
    }
}
