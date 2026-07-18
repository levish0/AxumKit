use anyhow::Result;
use config::{WorkerConfig, redact_database_url};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

/// Establishes and returns a database connection for the worker.
pub async fn establish_connection() -> Result<DatabaseConnection> {
    let config = WorkerConfig::get();

    info!(
        "Attempting to connect to database: {}",
        redact_database_url(&config.database_url)
    );

    // Configure connection options
    let mut options = ConnectOptions::new(config.database_url.clone());
    options
        .max_connections(WorkerConfig::get().db_max_connection)
        .min_connections(WorkerConfig::get().db_min_connection)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300))
        .sqlx_logging(false);

    Database::connect(options)
        .await
        .inspect(|_| {
            info!("Successfully connected to the database.");
        })
        .map_err(|err| {
            error!("Failed to connect to the database: {}", err);
            anyhow::Error::new(err).context("Failed to connect to the database")
        })
}
