use crate::config::WorkerConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

/// Establishes and returns a database connection for the worker.
pub async fn establish_connection() -> DatabaseConnection {
    let config = WorkerConfig::get();
    let database_url = config.database_url();

    info!("Attempting to connect to database...");

    // Configure connection options
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(WorkerConfig::get().db_write_max_connection)
        .min_connections(WorkerConfig::get().db_write_min_connection)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(false);

    match Database::connect(options).await {
        Ok(connection) => {
            info!("Successfully connected to the database.");
            connection
        }
        Err(err) => {
            error!("Failed to connect to the database: {}", err);
            panic!("Failed to connect to the database");
        }
    }
}
