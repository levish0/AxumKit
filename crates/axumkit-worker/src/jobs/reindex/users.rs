use super::common::DEFAULT_BATCH_SIZE;
use super::{ReindexJobBase, ReindexUsersJob};
use crate::jobs::WorkerContext;
use crate::jobs::index::user::{USERS_INDEX, build_user_search_json, ensure_index_settings};
use crate::nats::JetStreamContext;
use crate::nats::consumer::NatsConsumer;
use crate::nats::publisher::publish_job;
use crate::nats::streams::{REINDEX_USERS_CONSUMER, REINDEX_USERS_STREAM, REINDEX_USERS_SUBJECT};
use crate::{DbPool, SearchClient};
use axumkit_entity::users;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

/// Handle a batch reindex job for users
async fn handle_reindex_users(
    job: ReindexUsersJob,
    client: &SearchClient,
    db: &DbPool,
    jetstream: &JetStreamContext,
) -> Result<(), anyhow::Error> {
    tracing::info!(
        reindex_id = %job.base.reindex_id,
        batch_number = job.base.batch_number,
        after_id = ?job.base.after_id,
        batch_size = job.base.batch_size,
        "Processing user reindex batch"
    );

    // First batch: ensure index settings and clear existing data
    if job.base.after_id.is_none() {
        ensure_index_settings(client).await?;

        // Clear all existing users before reindexing
        let index = client.index(USERS_INDEX);
        index.delete_all_documents().await?;

        let total = users::Entity::find().count(db.as_ref()).await?;
        tracing::info!(
            reindex_id = %job.base.reindex_id,
            total_users = total,
            "Starting user reindex"
        );
    }

    // Fetch batch of users
    let users_batch =
        fetch_users_batch(db.as_ref(), job.base.after_id, job.base.batch_size).await?;

    if users_batch.is_empty() {
        tracing::info!(
            reindex_id = %job.base.reindex_id,
            total_batches = job.base.batch_number,
            "User reindex completed"
        );
        return Ok(());
    }

    // Build search documents
    let search_docs: Vec<_> = users_batch.iter().map(build_user_search_json).collect();

    // Index batch to MeiliSearch
    let index = client.index(USERS_INDEX);
    index.add_documents(&search_docs, Some("id")).await?;

    let processed_count = users_batch.len();
    let last_id = users_batch
        .last()
        .map(|u| u.id)
        .ok_or_else(|| anyhow::anyhow!("users_batch unexpectedly empty"))?;

    tracing::info!(
        reindex_id = %job.base.reindex_id,
        batch_number = job.base.batch_number,
        processed = processed_count,
        last_id = %last_id,
        "Batch processed"
    );

    // Self-enqueue next batch via NATS
    let next_job = ReindexUsersJob {
        base: ReindexJobBase {
            after_id: Some(last_id),
            batch_size: job.base.batch_size,
            reindex_id: job.base.reindex_id,
            batch_number: job.base.batch_number + 1,
        },
    };

    publish_job(jetstream, REINDEX_USERS_SUBJECT, &next_job).await?;

    Ok(())
}

/// Fetch a batch of users using UUID v7 cursor pagination
async fn fetch_users_batch(
    db: &sea_orm::DatabaseConnection,
    after_id: Option<Uuid>,
    batch_size: u32,
) -> Result<Vec<users::Model>, anyhow::Error> {
    let mut query = users::Entity::find().order_by_asc(users::Column::Id);

    if let Some(cursor) = after_id {
        query = query.filter(users::Column::Id.gt(cursor));
    }

    let users = query.limit(batch_size as u64).all(db).await?;

    Ok(users)
}

/// Create a new ReindexUsersJob to start reindexing from the beginning
pub fn create_reindex_users_job(reindex_id: Uuid, batch_size: Option<u32>) -> ReindexUsersJob {
    ReindexUsersJob {
        base: ReindexJobBase {
            after_id: None,
            batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
            reindex_id,
            batch_number: 1,
        },
    }
}

/// Run the reindex users consumer
pub async fn run_consumer(ctx: WorkerContext) -> anyhow::Result<()> {
    let meili_client = ctx.meili_client.clone();
    let db_pool = ctx.db_pool.clone();
    let jetstream = ctx.jetstream.clone();

    let consumer = NatsConsumer::new(
        ctx.jetstream.clone(),
        REINDEX_USERS_STREAM,
        REINDEX_USERS_CONSUMER,
        1, // concurrency
    );

    consumer
        .run::<ReindexUsersJob, _, _>(move |job| {
            let client = meili_client.clone();
            let db = db_pool.clone();
            let js = jetstream.clone();
            async move { handle_reindex_users(job, &client, &db, &js).await }
        })
        .await
}
