use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use once_cell::sync::OnceCell;
use std::env;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub is_dev: bool,

    pub jwt_secret: String,
    pub auth_access_token_expire_time: i64,
    pub auth_refresh_token_expire_time: i64,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_url: String,

    pub db_user: String,
    pub db_password: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_max_connection: u32,
    pub db_min_connection: u32,

    pub server_host: String,
    pub server_port: String,

    pub cors_allowed_origins: Vec<HeaderValue>,
    pub cors_allowed_headers: Vec<HeaderName>,
    pub cors_max_age: Option<u64>,
}

impl DbConfig {
    pub fn from_env() -> Self {
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

        Self {
            is_dev,
            jwt_secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),

            auth_access_token_expire_time: env::var("AUTH_ACCESS_TOKEN_EXPIRE_TIME")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30), // 기본값 24시간
            auth_refresh_token_expire_time: env::var("AUTH_REFRESH_TOKEN_EXPIRE_TIME")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(14), // 기본값 7일 (일주일)

            google_client_id: env::var("GOOGLE_CLIENT_ID").expect("GOOGLE_CLIENT_ID must be set"),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").expect("GOOGLE_CLIENT_SECRET must be set"),
            google_redirect_url: env::var("GOOGLE_REDIRECT_URL").expect("GOOGLE_REDIRECT_URL must be set"),

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
            server_host: env::var("HOST").expect("HOST must be set in .env file"),
            server_port: env::var("PORT").expect("PORT must be set in .env file"),
            cors_allowed_origins: cors_origins,
            cors_allowed_headers: cors_headers,
            cors_max_age: env::var("CORS_MAX_AGE").ok().and_then(|v| v.parse().ok()),
        }
    }

    pub fn init() {
        CONFIG
            .set(Self::from_env())
            .expect("DbConfig can only be initialized once");
    }

    pub fn get() -> &'static DbConfig {
        CONFIG
            .get()
            .expect("DbConfig is not initialized. Call DbConfig::init() first.")
    }
}

static CONFIG: OnceCell<DbConfig> = OnceCell::new();
