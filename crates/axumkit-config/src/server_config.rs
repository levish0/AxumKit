use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub is_dev: bool,

    pub totp_secret: String, // Secret for hashing TOTP backup codes
    pub auth_session_max_lifetime_hours: i64, // Maximum session lifetime (hours)
    pub auth_session_sliding_ttl_hours: i64, // Sliding TTL extended on activity (hours)
    pub auth_session_refresh_threshold: u8, // TTL refresh threshold (%)
    pub auth_email_verification_token_expire_time: i64, // minutes
    pub auth_password_reset_token_expire_time: i64, // minutes
    pub auth_email_change_token_expire_time: i64, // minutes
    pub oauth_pending_signup_ttl_minutes: i64, // OAuth pending signup TTL (minutes)

    // Google
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,

    // Github
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_uri: String,

    // Cloudflare R2 (shared credentials)
    pub r2_endpoint: String,
    pub r2_region: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    // R2 Assets (public bucket - images, sitemap)
    pub r2_assets_public_domain: String,
    pub r2_assets_bucket_name: String,

    // Cloudflare Turnstile
    pub turnstile_secret_key: String,

    // Database (via PgDog connection pooler)
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_max_connection: u32,
    pub db_min_connection: u32,

    // Redis Session (persistent, for sessions/tokens/rate-limit)
    pub redis_session_host: String,
    pub redis_session_port: String,

    // Redis Cache (volatile, for document cache)
    pub redis_cache_host: String,
    pub redis_cache_port: String,
    pub redis_cache_ttl: u64,

    pub server_host: String,
    pub server_port: String,

    // NATS (for background job queue)
    pub nats_url: String,

    // Meilisearch
    pub meilisearch_host: String,
    pub meilisearch_api_key: Option<String>,

    pub cors_allowed_origins: Vec<HeaderValue>,
    pub cors_allowed_headers: Vec<HeaderName>,
    pub cors_max_age: Option<u64>,

    // Cookie Domain (e.g., ".seven.wiki" for cross-subdomain cookies)
    pub cookie_domain: Option<String>,

    // Stability Layer (protect DB pool from overload)
    pub stability_concurrency_limit: usize, // Max concurrent requests (default: 500)
    pub stability_buffer_size: usize,       // Request queue size (default: 1024)
    pub stability_timeout_secs: u64,        // Request timeout in seconds (default: 30)
}

// Auto-initialized via LazyLock
static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| {
    dotenv().ok();

    let mut errors: Vec<String> = Vec::new();

    // Required env vars (collects errors on missing, does not panic)
    macro_rules! require {
        ($name:expr) => {
            env::var($name).unwrap_or_else(|_| {
                errors.push(format!("  - {} (missing)", $name));
                String::new()
            })
        };
    }

    // Required env vars + parsing (collects errors on missing/parse failure)
    macro_rules! require_parse {
        ($name:expr, $ty:ty) => {{
            match env::var($name) {
                Ok(raw) => match raw.parse::<$ty>() {
                    Ok(v) => v,
                    Err(_) => {
                        errors.push(format!("  - {} (invalid value: '{}')", $name, raw));
                        Default::default()
                    }
                },
                Err(_) => {
                    errors.push(format!("  - {} (missing)", $name));
                    Default::default()
                }
            }
        }};
    }

    let is_dev = matches!(
        env::var("ENVIRONMENT").as_deref(),
        Ok("dev") | Ok("development")
    );

    let cors_origins: Vec<HeaderValue> = match env::var("CORS_ALLOWED_ORIGINS").ok() {
        Some(origins) => origins
            .split(',')
            .filter_map(|s| {
                let trimmed_s = s.trim();
                if trimmed_s.is_empty() {
                    warn!("Empty origin found in CORS_ALLOWED_ORIGINS.");
                    None
                } else {
                    HeaderValue::from_str(trimmed_s).ok().or_else(|| {
                        warn!(
                            "Invalid origin '{}' found in CORS_ALLOWED_ORIGINS.",
                            trimmed_s
                        );
                        None
                    })
                }
            })
            .collect(),
        None => {
            vec![]
        }
    };

    let cors_headers: Vec<HeaderName> = match env::var("CORS_ALLOWED_HEADERS").ok() {
        Some(headers) => headers
            .split(',')
            .filter_map(|s| {
                let trimmed_s = s.trim();
                if trimmed_s.is_empty() {
                    warn!("Empty header name found in CORS_ALLOWED_HEADERS.");
                    None
                } else {
                    HeaderName::from_bytes(trimmed_s.as_bytes())
                        .ok()
                        .or_else(|| {
                            warn!(
                                "Invalid header name '{}' found in CORS_ALLOWED_HEADERS.",
                                trimmed_s
                            );
                            None
                        })
                }
            })
            .collect(),
        None => {
            vec![]
        }
    };

    // Required string vars
    let totp_secret = require!("TOTP_SECRET");
    let google_client_id = require!("GOOGLE_CLIENT_ID");
    let google_client_secret = require!("GOOGLE_CLIENT_SECRET");
    let google_redirect_uri = require!("GOOGLE_REDIRECT_URI");
    let github_client_id = require!("GITHUB_CLIENT_ID");
    let github_client_secret = require!("GITHUB_CLIENT_SECRET");
    let github_redirect_uri = require!("GITHUB_REDIRECT_URI");
    let r2_endpoint = require!("R2_ENDPOINT");
    let r2_region = require!("R2_REGION");
    let r2_access_key_id = require!("R2_ACCESS_KEY_ID");
    let r2_secret_access_key = require!("R2_SECRET_ACCESS_KEY");
    let r2_assets_public_domain = require!("R2_ASSETS_PUBLIC_DOMAIN");
    let r2_assets_bucket_name = require!("R2_ASSETS_BUCKET_NAME");
    let turnstile_secret_key = require!("TURNSTILE_SECRET_KEY");
    let db_host = require!("POSTGRES_HOST");
    let db_port = require!("POSTGRES_PORT");
    let db_name = require!("POSTGRES_NAME");
    let db_user = require!("POSTGRES_USER");
    let db_password = require!("POSTGRES_PASSWORD");
    let server_host = require!("HOST");
    let server_port = require!("PORT");

    // Required parsed vars
    let auth_session_max_lifetime_hours = require_parse!("AUTH_SESSION_MAX_LIFETIME_HOURS", i64);
    let auth_session_sliding_ttl_hours = require_parse!("AUTH_SESSION_SLIDING_TTL_HOURS", i64);
    let auth_session_refresh_threshold = require_parse!("AUTH_SESSION_REFRESH_THRESHOLD", u8);

    // Panic with all errors at once
    if !errors.is_empty() {
        panic!(
            "\n\nMissing or invalid environment variables ({} errors):\n{}\n",
            errors.len(),
            errors.join("\n")
        );
    }

    ServerConfig {
        is_dev,
        totp_secret,

        auth_session_max_lifetime_hours: auth_session_max_lifetime_hours.max(0),
        auth_session_sliding_ttl_hours: auth_session_sliding_ttl_hours.max(0),
        auth_session_refresh_threshold,

        auth_email_verification_token_expire_time: env::var(
            "AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME",
        )
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(1)
        .max(0), // Default 1 hour (minutes)
        auth_password_reset_token_expire_time: env::var("AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(15)
            .max(0), // Default 15 minutes
        auth_email_change_token_expire_time: env::var("AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(15)
            .max(0), // Default 15 minutes
        oauth_pending_signup_ttl_minutes: env::var("OAUTH_PENDING_SIGNUP_TTL_MINUTES")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(10)
            .max(0), // Default 10 minutes

        // Google
        google_client_id,
        google_client_secret,
        google_redirect_uri,

        // Github
        github_client_id,
        github_client_secret,
        github_redirect_uri,

        // Cloudflare R2 (shared credentials)
        r2_endpoint,
        r2_region,
        r2_access_key_id,
        r2_secret_access_key,
        // R2 Assets (public bucket)
        r2_assets_public_domain,
        r2_assets_bucket_name,

        // Cloudflare Turnstile
        turnstile_secret_key,

        // Database (via PgDog connection pooler)
        db_host,
        db_port,
        db_name,
        db_user,
        db_password,
        db_max_connection: env::var("POSTGRES_MAX_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
        db_min_connection: env::var("POSTGRES_MIN_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5),

        // Redis Session
        redis_session_host: env::var("REDIS_SESSION_HOST")
            .unwrap_or_else(|_| "redis-session".to_string()),
        redis_session_port: env::var("REDIS_SESSION_PORT").unwrap_or_else(|_| "6379".to_string()),

        // Redis Cache
        redis_cache_host: env::var("REDIS_CACHE_HOST")
            .unwrap_or_else(|_| "redis-cache".to_string()),
        redis_cache_port: env::var("REDIS_CACHE_PORT").unwrap_or_else(|_| "6379".to_string()),
        redis_cache_ttl: env::var("REDIS_CACHE_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600),

        server_host,
        server_port,

        // NATS
        nats_url: env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string()),

        // Meilisearch
        meilisearch_host: env::var("MEILISEARCH_HOST")
            .unwrap_or_else(|_| "http://localhost:7700".to_string()),
        meilisearch_api_key: env::var("MEILISEARCH_API_KEY")
            .ok()
            .filter(|key| !key.is_empty()),

        cors_allowed_origins: cors_origins,
        cors_allowed_headers: cors_headers,
        cors_max_age: env::var("CORS_MAX_AGE").ok().and_then(|v| v.parse().ok()),

        cookie_domain: env::var("COOKIE_DOMAIN").ok().filter(|d| !d.is_empty()),

        // Stability Layer
        stability_concurrency_limit: env::var("STABILITY_CONCURRENCY_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500),
        stability_buffer_size: env::var("STABILITY_BUFFER_SIZE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1024),
        stability_timeout_secs: env::var("STABILITY_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30),
    }
});

impl ServerConfig {
    // Simply access the CONFIG static
    pub fn get() -> &'static ServerConfig {
        &CONFIG
    }
}
