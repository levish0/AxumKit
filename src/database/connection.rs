use crate::config::db_config::DbConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

pub async fn establish_connection() -> DatabaseConnection {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        &DbConfig::get().db_user,
        &DbConfig::get().db_password,
        &DbConfig::get().db_host,
        &DbConfig::get().db_port,
        &DbConfig::get().db_name
    );
    info!("Attempting to connect to database: {}", database_url);

    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(DbConfig::get().db_max_connection)
        .min_connections(DbConfig::get().db_min_connection)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(true);

    match Database::connect(options).await {
        Ok(connection) => {
            info!("Successfully connected to the database.");
            connection
        }
        Err(err) => {
            // 연결 실패시 에러 로그 출력
            error!("Failed to connect to database: {}", err);
            panic!("Failed to connect to database");
        }
    }
}
