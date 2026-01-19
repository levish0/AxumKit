use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub is_dev: bool,

    pub totp_secret: String,                  // TOTP 백업 코드 해시용 시크릿
    pub auth_session_max_lifetime_hours: i64, // 세션 최대 수명 (시간)
    pub auth_session_sliding_ttl_hours: i64,  // 활동 시 연장 TTL (시간)
    pub auth_session_refresh_threshold: u8,   // TTL 갱신 임계값 (%)
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

    // Cloudflare
    pub r2_endpoint: String,
    pub r2_region: String,
    pub r2_public_domain: String,
    pub r2_bucket_name: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub turnstile_secret_key: String,

    pub db_user: String,
    pub db_password: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
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

    // SeaweedFS (revision content storage)
    pub seaweedfs_endpoint: String,

    pub cors_allowed_origins: Vec<HeaderValue>,
    pub cors_allowed_headers: Vec<HeaderName>,
    pub cors_max_age: Option<u64>,

    // Cookie Domain (e.g., ".example.com" for cross-subdomain cookies)
    pub cookie_domain: Option<String>,
}

// LazyLock으로 자동 초기화
static CONFIG: LazyLock<ServerConfig> = LazyLock::new(|| {
    dotenv().ok();

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

    ServerConfig {
        is_dev,
        totp_secret: env::var("TOTP_SECRET").expect("TOTP_SECRET must be set"),

        auth_session_max_lifetime_hours: env::var("AUTH_SESSION_MAX_LIFETIME_HOURS")
            .expect("AUTH_SESSION_MAX_LIFETIME_HOURS must be set")
            .parse()
            .expect("AUTH_SESSION_MAX_LIFETIME_HOURS must be a valid integer"),

        auth_session_sliding_ttl_hours: env::var("AUTH_SESSION_SLIDING_TTL_HOURS")
            .expect("AUTH_SESSION_SLIDING_TTL_HOURS must be set")
            .parse()
            .expect("AUTH_SESSION_SLIDING_TTL_HOURS must be a valid integer"),

        auth_session_refresh_threshold: env::var("AUTH_SESSION_REFRESH_THRESHOLD")
            .expect("AUTH_SESSION_REFRESH_THRESHOLD must be set")
            .parse()
            .expect("AUTH_SESSION_REFRESH_THRESHOLD must be a valid integer (0-100)"),

        auth_email_verification_token_expire_time: env::var(
            "AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME",
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1), // 기본값 1시간 (minutes)
        auth_password_reset_token_expire_time: env::var("AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15), // 기본값 15분
        auth_email_change_token_expire_time: env::var("AUTH_EMAIL_CHANGE_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15), // 기본값 15분
        oauth_pending_signup_ttl_minutes: env::var("OAUTH_PENDING_SIGNUP_TTL_MINUTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10), // 기본값 10분

        // Google
        google_client_id: env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
        google_client_secret: env::var("GOOGLE_CLIENT_SECRET")
            .expect("GOOGLE_CLIENT_SECRET must be set"),
        google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
            .expect("GOOGLE_REDIRECT_URI must be set"),

        // Github
        github_client_id: env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID must be set"),
        github_client_secret: env::var("GITHUB_CLIENT_SECRET")
            .expect("GITHUB_CLIENT_SECRET must be set"),
        github_redirect_uri: env::var("GITHUB_REDIRECT_URI")
            .expect("GITHUB_REDIRECT_URI must be set"),
        // Cloudflare
        r2_endpoint: env::var("R2_ENDPOINT").expect("R2_ENDPOINT must be set"),
        r2_region: env::var("R2_REGION").expect("R2_REGION must be set"),
        r2_public_domain: env::var("R2_PUBLIC_DOMAIN").expect("R2_PUBLIC_DOMAIN must be set"),
        r2_bucket_name: env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set"),
        r2_access_key_id: env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set"),
        r2_secret_access_key: env::var("R2_SECRET_ACCESS_KEY")
            .expect("R2_SECRET_ACCESS_KEY must be set"),
        turnstile_secret_key: env::var("TURNSTILE_SECRET_KEY")
            .expect("TURNSTILE_SECRET_KEY must be set"),

        db_user: env::var("POSTGRES_USER").expect("POSTGRES_USER must be set"),
        db_password: env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set"),
        db_host: env::var("POSTGRES_HOST").expect("POSTGRES_HOST must be set"),
        db_port: env::var("POSTGRES_PORT").expect("POSTGRES_PORT must be set"),
        db_name: env::var("POSTGRES_NAME").expect("POSTGRES_NAME must be set"),
        db_max_connection: env::var("POSTGRES_MAX_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(100),
        db_min_connection: env::var("POSTGRES_MIN_CONNECTION")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10),

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

        server_host: env::var("HOST").expect("HOST must be set in .env file"),
        server_port: env::var("PORT").expect("PORT must be set in .env file"),

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

        // SeaweedFS
        seaweedfs_endpoint: env::var("SEAWEEDFS_ENDPOINT").expect("SEAWEEDFS_ENDPOINT must be set"),
    }
});

impl ServerConfig {
    // 이제 단순히 CONFIG에 접근만 하면 됨
    pub fn get() -> &'static ServerConfig {
        &CONFIG
    }
}
