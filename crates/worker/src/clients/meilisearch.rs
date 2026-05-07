use crate::SearchClient;
use crate::config::WorkerConfig;
use meilisearch_sdk::client::Client as MeiliClient;
use std::sync::Arc;

pub fn create_meili_client(config: &WorkerConfig) -> SearchClient {
    Arc::new(
        MeiliClient::new(&config.meilisearch_host, config.meilisearch_api_key.clone())
            .expect("Failed to create MeiliSearch client"),
    )
}
