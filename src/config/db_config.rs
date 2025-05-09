use dotenvy::dotenv;
use std::env;
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
}

impl DbConfig {
    pub fn from_env() -> Self {
        dotenv().ok();

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
        }
    }
}
