use axum::http::{HeaderName, HeaderValue};
use dotenvy::dotenv;
use std::env;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub db_user: String,
    pub db_password: String,
    pub db_host: String,
    pub db_port: String,
    pub db_name: String,
    pub db_max_connection: u32,
    pub db_min_connection: u32,

    pub server_host: String, // 서버 host
    pub server_port: String, // 서버 port

    pub cors_allowed_origins: Vec<HeaderValue>,
    pub cors_allowed_headers: Vec<HeaderName>,
}

impl DbConfig {
    pub fn from_env() -> Self {
        dotenv().ok();

        let cors_origins = match env::var("CORS_ALLOWED_ORIGINS").ok() {
            Some(origins) => {
                let parsed: Vec<HeaderValue> = origins
                    .split(',')
                    .map(|s| HeaderValue::from_str(s.trim()).unwrap())
                    .collect();

                if parsed.is_empty() {
                    warn!(
                        "CORS_ALLOWED_ORIGINS is set but empty. Did you forget to add any origins?"
                    );
                }

                parsed
            }
            None => {
                warn!(
                    "CORS_ALLOWED_ORIGINS is not set. Did you forget to define it in the .env file?"
                );
                vec![]
            }
        };

        let cors_headers = match env::var("CORS_ALLOWED_HEADERS").ok() {
            Some(headers) => {
                let parsed: Vec<HeaderName> = headers
                    .split(',')
                    .map(|s| HeaderName::from_bytes(s.trim().as_bytes()).unwrap())
                    .collect();

                if parsed.is_empty() {
                    warn!(
                        "CORS_ALLOWED_HEADERS is set but empty. Did you forget to add any headers?"
                    );
                }

                parsed
            }
            None => {
                warn!(
                    "CORS_ALLOWED_HEADERS is not set. Did you forget to define it in the .env file?"
                );
                vec![]
            }
        };

        Self {
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
        }
    }
}
