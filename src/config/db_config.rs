use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub is_dev: bool,

    pub jwt_secret: String,
    pub auth_session_expire_time: i64,                  // hours
    pub auth_email_verification_token_expire_time: i64, // hours
    pub auth_password_reset_token_expire_time: i64,     // hours

    // Google
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,

    // Github
    pub github_client_id: String,
    pub github_client_secret: String,
    pub github_redirect_uri: String,

    // Cloudflare
    pub r2_public_domain: String,
    pub r2_account_id: String,
    pub r2_bucket_name: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,

    pub db_user: String,
    pub db_password: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_max_connection: u32,
    pub db_min_connection: u32,

    // Redis
    pub redis_host: String,
    pub redis_port: String,
    pub redis_ttl: u64,

    pub server_host: String,
    pub server_port: String,

    pub cors_allowed_origins: Vec<HeaderValue>,
    pub cors_allowed_headers: Vec<HeaderName>,
    pub cors_max_age: Option<u64>,
}

// LazyLock으로 자동 초기화
static CONFIG: LazyLock<DbConfig> = LazyLock::new(|| {
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

    DbConfig {
        is_dev,
        jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),

        auth_session_expire_time: env::var("AUTH_SESSION_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(24), // 기본값 24시간 (hours)

        auth_email_verification_token_expire_time: env::var(
            "AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME",
        )
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1), // 기본값 1시간 (hours)
        auth_password_reset_token_expire_time: env::var("AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1), // 기본값 1시간 (hours)

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
        r2_public_domain: env::var("R2_PUBLIC_DOMAIN").expect("R2_PUBLIC_DOMAIN must be set"),
        r2_account_id: env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set"),
        r2_bucket_name: env::var("R2_BUCKET_NAME").expect("R2_BUCKET_NAME must be set"),
        r2_access_key_id: env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set"),
        r2_secret_access_key: env::var("R2_SECRET_ACCESS_KEY")
            .expect("R2_SECRET_ACCESS_KEY must be set"),

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

        // Redis
        redis_host: env::var("REDIS_HOST").expect("REDIS_HOST must be set"),
        redis_port: env::var("REDIS_PORT").expect("REDIS_PORT must be set"),
        redis_ttl: env::var("REDIS_TTL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3600),

        server_host: env::var("HOST").expect("HOST must be set in .env file"),
        server_port: env::var("PORT").expect("PORT must be set in .env file"),

        cors_allowed_origins: cors_origins,
        cors_allowed_headers: cors_headers,
        cors_max_age: env::var("CORS_MAX_AGE").ok().and_then(|v| v.parse().ok()),
    }
});

impl DbConfig {
    // 이제 단순히 CONFIG에 접근만 하면 됨
    pub fn get() -> &'static DbConfig {
        &CONFIG
    }
}
