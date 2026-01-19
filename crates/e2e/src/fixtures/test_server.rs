//! TestServer - main entry point for E2E tests

use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};

use super::api_client::ApiClient;
use super::containers::E2eContainers;

/// Main test application that manages all containers and provides API client
pub struct TestServer {
    pub client: ApiClient,
    /// Direct database connection for test setup/assertions
    pub db: DatabaseConnection,
    containers: E2eContainers,
}

impl TestServer {
    /// Start all containers and create test app
    pub async fn start() -> Result<Self> {
        // Start all containers via docker-compose
        tracing::info!("Starting E2E test environment...");
        let containers = E2eContainers::start().await?;

        // Connect to database (dynamically allocated port)
        let db_url = format!(
            "postgres://postgres:postgrespw@localhost:{}/sevenwiki",
            containers.postgres_port
        );
        tracing::info!("Connecting to database at {}", db_url);
        let db = Database::connect(&db_url).await?;

        // Create API client
        let client = ApiClient::new(&containers.base_url);

        Ok(Self {
            containers,
            client,
            db,
        })
    }

    /// Get base URL of the API server
    pub fn base_url(&self) -> &str {
        &self.containers.base_url
    }

    /// Get test server port (parsed from base_url)
    pub fn port(&self) -> u16 {
        self.containers
            .base_url
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8000)
    }

    /// Create an authenticated client
    pub fn authenticated_client(&self, session_cookie: &str) -> ApiClient {
        ApiClient::with_session(&self.containers.base_url, session_cookie)
    }

    /// Run database migration
    pub async fn run_migration(&self) -> Result<()> {
        self.containers.run_migration().await
    }

    /// Run database seed (ACL rules, etc.)
    pub async fn run_seed(&self) -> Result<()> {
        self.containers.run_seed().await
    }

    /// Get recent server logs (last N lines)
    pub async fn get_server_logs(&self, lines: u32) -> Result<String> {
        self.containers.get_server_logs(lines).await
    }

    /// Print server logs to test output
    pub async fn print_server_logs(&self, lines: u32) {
        match self.get_server_logs(lines).await {
            Ok(logs) => {
                tracing::info!("=== SERVER LOGS (last {} lines) ===\n{}", lines, logs);
            }
            Err(e) => {
                tracing::error!("Failed to get server logs: {}", e);
            }
        }
    }

    /// Stop all containers
    pub async fn stop(mut self) -> Result<()> {
        tracing::info!("Stopping containers...");
        self.containers.cleanup().await?;
        Ok(())
    }
}
