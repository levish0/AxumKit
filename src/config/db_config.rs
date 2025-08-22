use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use std::env;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub is_dev: bool,

    pub jwt_secret: String,
    pub auth_access_token_expire_time: i64,
    pub auth_refresh_token_expire_time: i64,
    pub auth_email_verification_token_expire_time: i64,
    pub auth_password_reset_token_expire_time: i64,

    // Google
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub google_link_redirect_uri: String,

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

    // OpenSearch
    // pub opensearch_host: String,
    // pub opensearch_port: String,
    // pub opensearch_scheme: String,
    // pub opensearch_username: String,
    // pub opensearch_password: String,
    // pub opensearch_verify_certs: bool,
    pub server_host: String,
    pub server_port: String,

    // Task Server
    pub task_server_host: String,
    pub task_server_port: String,

    // Meilisearch
    pub meilisearch_host: String,
    pub meilisearch_api_key: Option<String>,

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

        auth_access_token_expire_time: env::var("AUTH_ACCESS_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30), // 기본값 30분
        auth_refresh_token_expire_time: env::var("AUTH_REFRESH_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(14), // 기본값 14일 (일주일)
        auth_email_verification_token_expire_time: env::var(
            "AUTH_EMAIL_VERIFICATION_TOKEN_EXPIRE_TIME",
        )
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1), // 기본값 1시간
        auth_password_reset_token_expire_time: env::var("AUTH_PASSWORD_RESET_TOKEN_EXPIRE_TIME")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1), // 기본값 1시간

        // Google
        google_client_id: env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
        google_client_secret: env::var("GOOGLE_CLIENT_SECRET")
            .expect("GOOGLE_CLIENT_SECRET must be set"),
        google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
            .expect("GOOGLE_REDIRECT_URI must be set"),
        google_link_redirect_uri: env::var("GOOGLE_LINK_REDIRECT_URI")
            .expect("GOOGLE_LINK_REDIRECT_URI must be set"),

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

        // Opensearch
        /*
         opensearch_host: env::var("OPENSEARCH_HOST").expect("OPENSEARCH_HOST must be set"),
         opensearch_port: env::var("OPENSEARCH_PORT").expect("OPENSEARCH_HOST must be set"),
         opensearch_scheme: env::var("OPENSEARCH_SCHEME")
           .ok()
            .unwrap_or("http".to_string()),
         opensearch_username: env::var("OPENSEARCH_USERNAME").expect("OPENSEARCH_USERNAME must be set"),
         opensearch_password:env::var("OPENSEARCH_PASSWORD").expect("OPENSEARCH_PASSWORD must be set"),
        opensearch_verify_certs: env::var("OPENSEARCH_VERIFY_CERTS")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false),
        */
        server_host: env::var("HOST").expect("HOST must be set in .env file"),
        server_port: env::var("PORT").expect("PORT must be set in .env file"),

        // Task Server
        task_server_host: env::var("TASK_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        task_server_port: env::var("TASK_SERVER_PORT").unwrap_or_else(|_| "7000".to_string()),

        // Meilisearch
        meilisearch_host: env::var("MEILISEARCH_HOST")
            .unwrap_or_else(|_| "http://localhost:7700".to_string()),
        meilisearch_api_key: env::var("MEILISEARCH_API_KEY")
            .ok()
            .filter(|key| !key.is_empty()),

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