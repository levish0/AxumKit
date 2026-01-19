use super::common::DEFAULT_BATCH_SIZE;
use super::{ReindexJobBase, ReindexPostsJob};
use crate::jobs::WorkerContext;
use crate::jobs::index::post::{POSTS_INDEX, build_post_search_json, ensure_index_settings};
use crate::nats::JetStreamContext;
use crate::nats::consumer::NatsConsumer;
use crate::nats::publisher::publish_job;
use crate::nats::streams::{REINDEX_POSTS_CONSUMER, REINDEX_POSTS_STREAM, REINDEX_POSTS_SUBJECT};
use crate::{DbPool, SearchClient, StorageClient};
use axumkit_entity::posts;
use futures::future::join_all;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
use uuid::Uuid;

/// Handle a batch reindex job for posts
async fn handle_reindex_posts(
    job: ReindexPostsJob,
    client: &SearchClient,
    db: &DbPool,
    storage: &StorageClient,
    jetstream: &JetStreamContext,
) -> Result<(), anyhow::Error> {
    tracing::info!(
        reindex_id = %job.base.reindex_id,
        batch_number = job.base.batch_number,
        after_id = ?job.base.after_id,
        batch_size = job.base.batch_size,
        "Processing post reindex batch"
    );

    // First batch: ensure index settings and clear existing data
    if job.base.after_id.is_none() {
        ensure_index_settings(client).await?;

        // Clear all existing posts before reindexing
        let index = client.index(POSTS_INDEX);
        index.delete_all_documents().await?;

        let total = posts::Entity::find().count(db.as_ref()).await?;
        tracing::info!(
            reindex_id = %job.base.reindex_id,
            total_posts = total,
            "Starting post reindex"
        );
    }

    // Fetch batch of posts
    let post_list = fetch_posts_batch(db.as_ref(), job.base.after_id, job.base.batch_size).await?;

    if post_list.is_empty() {
        tracing::info!(
            reindex_id = %job.base.reindex_id,
            total_batches = job.base.batch_number,
            "Post reindex completed"
        );
        return Ok(());
    }

    // Fetch content from SeaweedFS concurrently
    let content_futures: Vec<_> = post_list
        .iter()
        .map(|p| {
            let storage = storage.clone();
            let storage_key = p.storage_key.clone();
            let id = p.id;
            async move {
                let content = storage.download_content(&storage_key).await;
                (id, content)
            }
        })
        .collect();

    let content_results = join_all(content_futures).await;
    let mut contents: HashMap<Uuid, String> = HashMap::new();
    for (id, result) in content_results {
        match result {
            Ok(content) => {
                contents.insert(id, content);
            }
            Err(e) => {
                tracing::warn!(post_id = %id, error = %e, "Failed to download content");
            }
        }
    }

    // Build search documents
    let mut search_docs = Vec::with_capacity(post_list.len());
    let mut failed_count = 0;

    for post in &post_list {
        let Some(content) = contents.get(&post.id) else {
            tracing::warn!(
                post_id = %post.id,
                "Content not found, skipping"
            );
            failed_count += 1;
            continue;
        };

        let search_doc = build_post_search_json(post, content);
        search_docs.push(search_doc);
    }

    // Index batch to MeiliSearch
    if !search_docs.is_empty() {
        let index = client.index(POSTS_INDEX);
        index.add_documents(&search_docs, Some("id")).await?;
    }

    let processed_count = post_list.len();
    let last_id = post_list
        .last()
        .map(|p| p.id)
        .ok_or_else(|| anyhow::anyhow!("posts unexpectedly empty"))?;

    tracing::info!(
        reindex_id = %job.base.reindex_id,
        batch_number = job.base.batch_number,
        processed = processed_count,
        failed = failed_count,
        last_id = %last_id,
        "Batch processed"
    );

    // Self-enqueue next batch via NATS
    let next_job = ReindexPostsJob {
        base: ReindexJobBase {
            after_id: Some(last_id),
            batch_size: job.base.batch_size,
            reindex_id: job.base.reindex_id,
            batch_number: job.base.batch_number + 1,
        },
    };

    publish_job(jetstream, REINDEX_POSTS_SUBJECT, &next_job).await?;

    Ok(())
}

/// Fetch a batch of posts using UUID v7 cursor pagination
async fn fetch_posts_batch(
    db: &sea_orm::DatabaseConnection,
    after_id: Option<Uuid>,
    batch_size: u32,
) -> Result<Vec<posts::Model>, anyhow::Error> {
    let mut query = posts::Entity::find().order_by_asc(posts::Column::Id);

    if let Some(cursor) = after_id {
        query = query.filter(posts::Column::Id.gt(cursor));
    }

    let post_list = query.limit(batch_size as u64).all(db).await?;

    Ok(post_list)
}

/// Create a new ReindexPostsJob to start reindexing from the beginning
pub fn create_reindex_posts_job(reindex_id: Uuid, batch_size: Option<u32>) -> ReindexPostsJob {
    ReindexPostsJob {
        base: ReindexJobBase {
            after_id: None,
            batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
            reindex_id,
            batch_number: 1,
        },
    }
}

/// Run the reindex posts consumer
pub async fn run_consumer(ctx: WorkerContext) -> anyhow::Result<()> {
    let meili_client = ctx.meili_client.clone();
    let db_pool = ctx.db_pool.clone();
    let storage_client = ctx.storage_client.clone();
    let jetstream = ctx.jetstream.clone();

    let consumer = NatsConsumer::new(
        ctx.jetstream.clone(),
        REINDEX_POSTS_STREAM,
        REINDEX_POSTS_CONSUMER,
        1, // concurrency
    );

    consumer
        .run::<ReindexPostsJob, _, _>(move |job| {
            let client = meili_client.clone();
            let db = db_pool.clone();
            let storage = storage_client.clone();
            let js = jetstream.clone();
            async move { handle_reindex_posts(job, &client, &db, &storage, &js).await }
        })
        .await
}
