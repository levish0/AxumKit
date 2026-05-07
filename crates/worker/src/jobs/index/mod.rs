pub mod user;

// Re-export job types and handlers for external use
pub use user::{
    IndexUserJob, USERS_INDEX, UserIndexAction, build_user_search_json,
    ensure_index_settings as ensure_user_index_settings,
};

/// Initialize all MeiliSearch indexes on worker startup.
/// This ensures indexes exist before any search queries are made.
pub async fn initialize_all_indexes(
    client: &meilisearch_sdk::client::Client,
) -> Result<(), anyhow::Error> {
    tracing::info!("Initializing MeiliSearch indexes...");

    ensure_user_index_settings(client).await?;
    tracing::info!("Users index ready");

    tracing::info!("All MeiliSearch indexes initialized");
    Ok(())
}
