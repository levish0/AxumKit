use axumkit_config::ServerConfig;
use meilisearch_sdk::client::Client;

#[derive(Debug, Clone)]
pub struct MeilisearchClient {
    client: Client,
}

impl MeilisearchClient {
    pub fn new() -> Result<Self, meilisearch_sdk::errors::Error> {
        let config = ServerConfig::get();

        let client = if let Some(api_key) = &config.meilisearch_api_key {
            Client::new(&config.meilisearch_host, Some(api_key.as_str()))?
        } else {
            Client::new(&config.meilisearch_host, None::<&str>)?
        };

        Ok(MeilisearchClient { client })
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }
}
