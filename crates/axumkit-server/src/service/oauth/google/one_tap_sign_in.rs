use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::service::auth::verify_email::find_pending_email_signup_by_email;
use crate::service::oauth::types::PendingSignupData;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use axumkit_config::ServerConfig;
use axumkit_constants::oauth_pending_key;
use axumkit_dto::oauth::internal::SignInResult;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use redis::aio::ConnectionManager;
use reqwest::header::{CACHE_CONTROL, HeaderValue};
use sea_orm::ConnectionTrait;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const DEFAULT_JWKS_CACHE_TTL_SECONDS: u64 = 300;

static GOOGLE_JWKS_CACHE: LazyLock<RwLock<Option<CachedGoogleJwks>>> =
    LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
struct CachedGoogleJwks {
    jwks: JwkSet,
    expires_at: Instant,
}

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    sub: String,
    email: String,
    email_verified: bool,
    picture: Option<String>,
}

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

    // 1. Check whether this Google identity is already linked.
    if let Some(existing_user) =
        repository_find_user_by_oauth(conn, OAuthProvider::Google, &token_data.claims.sub).await?
    {
        let session = SessionService::create_session(
            redis_conn,
            existing_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        return Ok(SignInResult::Success(session.session_id));
    }

    // 2. New user path: reject if the email already belongs to another account.
    if repository_find_user_by_email(conn, token_data.claims.email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 2b. Check if a pending email/password signup holds this email
    if find_pending_email_signup_by_email(redis_conn, &token_data.claims.email)
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 3. New user path: store pending-signup data in Redis.
    let config = ServerConfig::get();
    let pending_data = PendingSignupData {
        provider: OAuthProvider::Google,
        provider_user_id: token_data.claims.sub,
        anonymous_user_id: anonymous_user_id.to_string(),
        email: token_data.claims.email.clone(),
        profile_image: token_data.claims.picture,
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
        email: token_data.claims.email,
    })
}

async fn get_google_jwks(
    http_client: &reqwest::Client,
    force_refresh: bool,
) -> ServiceResult<(JwkSet, bool)> {
    let now = Instant::now();
    if !force_refresh {
        let cache = GOOGLE_JWKS_CACHE.read().await;
        if let Some(cached) = cache.as_ref() {
            if now < cached.expires_at {
                return Ok((cached.jwks.clone(), true));
            }
        }
    }

    // Acquire write lock and double-check to prevent cache stampede
    let mut cache = GOOGLE_JWKS_CACHE.write().await;
    if !force_refresh {
        if let Some(cached) = cache.as_ref() {
            if Instant::now() < cached.expires_at {
                return Ok((cached.jwks.clone(), true));
            }
        }
    }

    let (jwks, cache_ttl_seconds) = fetch_google_jwks(http_client).await?;
    *cache = Some(CachedGoogleJwks {
        jwks: jwks.clone(),
        expires_at: Instant::now() + Duration::from_secs(cache_ttl_seconds),
    });

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
