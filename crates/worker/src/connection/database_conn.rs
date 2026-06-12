use anyhow::Result;
use config::WorkerConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

/// Establishes and returns a database connection for the worker.
pub async fn establish_connection() -> Result<DatabaseConnection> {
    let config = WorkerConfig::get();
    let database_url = config.database_url();

    info!("Attempting to connect to database...");

    // Configure connection options
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(config.db_max_connection)
        .min_connections(config.db_min_connection)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(false);

    Database::connect(options)
        .await
        .inspect(|_connection| {
            info!("Successfully connected to the database.");
        })
        .map_err(|err| {
            error!("Failed to connect to the database: {}", err);
            anyhow::Error::new(err).context("Failed to connect to the database")
        })
}
