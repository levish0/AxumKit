use crate::jobs::WorkerContext;
use crate::nats::consumer::NatsConsumer;
use crate::nats::streams::{INDEX_POST_CONSUMER, INDEX_POST_STREAM};
use crate::{DbPool, SearchClient, StorageClient};
use axumkit_entity::posts;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPostJob {
    pub post_id: Uuid,
    pub action: IndexAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexAction {
    Index,
    Delete,
}

pub const POSTS_INDEX: &str = "posts";

/// Build a search document JSON from post metadata and content
pub fn build_post_search_json(post: &posts::Model, content: &str) -> JsonValue {
    json!({
        "id": post.id.to_string(),
        "author_id": post.author_id.to_string(),
        "title": post.title,
        "content": content,
        "created_at": post.created_at.timestamp(),
        "updated_at": post.updated_at.timestamp(),
    })
}

/// MeiliSearch index settings for posts
fn post_index_settings() -> meilisearch_sdk::settings::Settings {
    meilisearch_sdk::settings::Settings::new()
        .with_searchable_attributes(["title", "content"])
        .with_filterable_attributes(["author_id"])
        .with_displayed_attributes([
            "id",
            "author_id",
            "title",
            "content",
            "created_at",
            "updated_at",
        ])
        .with_ranking_rules(["words", "typo", "proximity", "attribute", "exactness"])
}

async fn handle_index_post(
    job: IndexPostJob,
    client: &SearchClient,
    db: &DbPool,
    storage: &StorageClient,
) -> Result<(), anyhow::Error> {
    tracing::info!(
        "Processing post index job: post_id={}, action={:?}",
        job.post_id,
        job.action
    );

    let index = client.index(POSTS_INDEX);

    // Ensure index exists and settings are applied
    ensure_index_settings(client).await?;

    match job.action {
        IndexAction::Index => {
            // Fetch post from DB
            let post = posts::Entity::find_by_id(job.post_id)
                .one(db.as_ref())
                .await?
                .ok_or_else(|| anyhow::anyhow!("Post not found: {}", job.post_id))?;

            // Fetch content from SeaweedFS
            let content = storage
                .download_content(&post.storage_key)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to download content: {}", e))?;

            // Build search document
            let search_doc = build_post_search_json(&post, &content);

            // Add to index (upsert)
            index.add_documents(&[search_doc], Some("id")).await?;
            tracing::info!("Post {} indexed successfully", job.post_id);
        }
        IndexAction::Delete => {
            index.delete_document(&job.post_id.to_string()).await?;
            tracing::info!("Post {} deleted from index", job.post_id);
        }
    }

    Ok(())
}

/// Ensure index exists with proper settings
pub async fn ensure_index_settings(
    client: &meilisearch_sdk::client::Client,
) -> Result<(), anyhow::Error> {
    let index = client.index(POSTS_INDEX);

    // Check if index exists by trying to get stats
    match index.get_stats().await {
        Ok(_) => {
            // Index exists, settings should already be applied
            Ok(())
        }
        Err(meilisearch_sdk::errors::Error::Meilisearch(ref e))
            if e.error_code == meilisearch_sdk::errors::ErrorCode::IndexNotFound =>
        {
            // Index doesn't exist, create it
            tracing::info!("Creating posts index...");
            let task = client.create_index(POSTS_INDEX, Some("id")).await?;
            task.wait_for_completion(client, None, None).await?;

            // Apply settings
            tracing::info!("Applying posts index settings...");
            let index = client.index(POSTS_INDEX);
            let task = index.set_settings(&post_index_settings()).await?;
            task.wait_for_completion(client, None, None).await?;

            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Run the post index consumer
pub async fn run_consumer(ctx: WorkerContext) -> anyhow::Result<()> {
    let meili_client = ctx.meili_client.clone();
    let db_pool = ctx.db_pool.clone();
    let storage_client = ctx.storage_client.clone();

    let consumer = NatsConsumer::new(
        ctx.jetstream.clone(),
        INDEX_POST_STREAM,
        INDEX_POST_CONSUMER,
        4, // concurrency
    );

    consumer
        .run::<IndexPostJob, _, _>(move |job| {
            let client = meili_client.clone();
            let db = db_pool.clone();
            let storage = storage_client.clone();
            async move { handle_index_post(job, &client, &db, &storage).await }
        })
        .await
}
