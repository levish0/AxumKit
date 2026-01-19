use axumkit_config::ServerConfig;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use tracing::{error, info};

// This module manages database connections.
// It provides functionality for connecting to a PostgreSQL database
// and configuring the connection pool.

/// Establishes and returns a database connection.
///
/// This function is called at application startup to connect to the PostgreSQL database.
/// It reads connection information from the configuration file and sets up the connection pool.
///
/// # Returns
/// * `DatabaseConnection` - The successfully established database connection object.
pub async fn establish_connection() -> DatabaseConnection {
    // Retrieve database connection information from the environment and build the URL
    let db_config = ServerConfig::get();
    let database_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        &db_config.db_user,
        &db_config.db_password,
        &db_config.db_host,
        &db_config.db_port,
        &db_config.db_name
    );

    // Log with masked password
    let masked_url = format!(
        "postgres://{}:{}@{}:{}/{}",
        &db_config.db_user,
        "*".repeat(db_config.db_password.len()),
        &db_config.db_host,
        &db_config.db_port,
        &db_config.db_name
    );
    info!("Attempting to connect to database: {}", masked_url);

    // Configure connection options
    let mut options = ConnectOptions::new(database_url);
    options
        // Configure connection pool size
        .max_connections(ServerConfig::get().db_max_connection) // Maximum number of connections
        .min_connections(ServerConfig::get().db_min_connection) // Minimum number of connections
        // Configure timeouts
        .connect_timeout(Duration::from_secs(8)) // Connection timeout: 8 seconds
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(300)) // Idle timeout: 5 minutes
        // Enable SQL logging (for debugging)
        .sqlx_logging(false);

    // Attempt to connect to the database and handle the result
    match Database::connect(options).await {
        Ok(connection) => {
            // On successful connection, log the success and return the connection object
            info!("Successfully connected to the database.");
            connection
        }
        Err(err) => {
            // On failure, log the error and terminate the application
            // Since the database connection is critical, the application cannot continue without it
            error!("Failed to connect to the database: {}", err);
            panic!("Failed to connect to the database");
        }
    }
}
