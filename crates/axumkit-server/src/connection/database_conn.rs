use axumkit_config::ServerConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

struct DbConnConfig<'a> {
    user: &'a str,
    password: &'a str,
    host: &'a str,
    port: &'a str,
    name: &'a str,
    max_connections: u32,
    min_connections: u32,
}

async fn establish_connection_with_config(
    config: DbConnConfig<'_>,
    label: &str,
) -> DatabaseConnection {
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.user, config.password, config.host, config.port, config.name
    );

    // Log with masked password
    let masked_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        config.user,
        "*".repeat(config.password.len()),
        config.host,
        config.port,
        config.name
    );
    info!(
        "Attempting to connect to {} database: {}",
        label, masked_url
    );

    // Configure connection options
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(false);

    match Database::connect(options).await {
        Ok(connection) => {
            info!("Successfully connected to the {} database.", label);
            connection
        }
        Err(err) => {
            error!("Failed to connect to the {} database: {}", label, err);
            panic!("Failed to connect to the {} database", label);
        }
    }
}

/// Establishes and returns a database connection (via PgDog connection pooler).
pub async fn establish_connection() -> DatabaseConnection {
    let db_config = ServerConfig::get();
    establish_connection_with_config(
        DbConnConfig {
            user: &db_config.db_user,
            password: &db_config.db_password,
            host: &db_config.db_host,
            port: &db_config.db_port,
            name: &db_config.db_name,
            max_connections: db_config.db_max_connection,
            min_connections: db_config.db_min_connection,
        },
        "PostgreSQL",
    )
    .await
}
