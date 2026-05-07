use sea_orm::DatabaseConnection;

/// Run the cleanup job
pub async fn run_cleanup(_db: &DatabaseConnection) {
    tracing::info!("Starting scheduled cleanup job");
    // Add cleanup tasks here as needed
    tracing::info!("Cleanup job completed");
}
