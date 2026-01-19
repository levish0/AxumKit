use anyhow::Result;
use axumkit_worker::clients;
use axumkit_worker::config::WorkerConfig;
use axumkit_worker::connection;
use axumkit_worker::jobs::{self, WorkerContext};
use axumkit_worker::nats::streams::initialize_all_streams;
use axumkit_worker::utils;
use axumkit_worker::{CacheClient, DbPool, StorageClient};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize config (loads .env internally)
    let config = WorkerConfig::get();

    // Initialize logging
    utils::logger::init_tracing();

    info!("Starting axumkit-worker...");

    // Create shared clients
    let mailer = clients::create_mailer(config)?;
    let meili_client = clients::create_meili_client(config);

    // Initialize MeiliSearch indexes (ensures they exist before any search queries)
    jobs::index::initialize_all_indexes(&meili_client).await?;

    // Establish database connection
    info!("Connecting to database...");
    let db_conn = connection::establish_connection().await;
    let db_pool: DbPool = Arc::new(db_conn);

    info!("Shared clients created");

    // Connect to Redis Cache (for view counts, etc.)
    let redis_cache_url = config.redis_cache_url();
    info!("Connecting to Redis Cache: {}", redis_cache_url);
    let redis_cache_client = redis::Client::open(redis_cache_url)?;
    let redis_cache_conn = redis::aio::ConnectionManager::new(redis_cache_client).await?;
    let cache_client: CacheClient = Arc::new(redis_cache_conn);

    // Connect to SeaweedFS (for revision content)
    info!("Connecting to SeaweedFS...");
    let seaweedfs_client = connection::establish_seaweedfs_connection(config).await?;
    let storage_client: StorageClient = Arc::new(seaweedfs_client);

    // Connect to R2 (for sitemap storage)
    info!("Connecting to R2...");
    let r2_client = connection::establish_r2_connection(config).await?;

    // Connect to NATS
    info!("Connecting to NATS: {}", config.nats_url);
    let nats_client = async_nats::connect(&config.nats_url).await?;
    let jetstream = async_nats::jetstream::new(nats_client);
    let jetstream = Arc::new(jetstream);

    // Initialize JetStream streams (creates if not exist)
    info!("Initializing JetStream streams...");
    initialize_all_streams(&jetstream).await?;

    // Create worker context with all shared resources
    let ctx = WorkerContext {
        mailer,
        meili_client,
        db_pool,
        cache_client,
        storage_client,
        r2_client,
        jetstream,
        config,
    };

    info!("Starting job consumers...");

    // Spawn all job consumers
    let email_handle = tokio::spawn(jobs::email::run_consumer(ctx.clone()));
    let index_post_handle = tokio::spawn(jobs::index::post::run_consumer(ctx.clone()));
    let index_user_handle = tokio::spawn(jobs::index::user::run_consumer(ctx.clone()));
    let reindex_posts_handle = tokio::spawn(jobs::reindex::posts::run_consumer(ctx.clone()));
    let reindex_users_handle = tokio::spawn(jobs::reindex::users::run_consumer(ctx.clone()));
    let delete_content_handle = tokio::spawn(jobs::storage::run_consumer(ctx.clone()));

    info!("Job consumers started");

    // Start cron scheduler (tokio-cron-scheduler)
    info!("Starting cron scheduler...");
    let _cron_scheduler = jobs::cron::start_scheduler(
        ctx.db_pool.clone(),
        ctx.r2_client.clone(),
        ctx.storage_client.as_ref().clone(),
        config,
    )
    .await?;

    info!("All workers running");

    // Wait for all tasks (they run indefinitely)
    tokio::select! {
        result = email_handle => {
            if let Err(e) = result {
                tracing::error!("Email consumer panicked: {:?}", e);
            }
        }
        result = index_post_handle => {
            if let Err(e) = result {
                tracing::error!("Index post consumer panicked: {:?}", e);
            }
        }
        result = index_user_handle => {
            if let Err(e) = result {
                tracing::error!("Index user consumer panicked: {:?}", e);
            }
        }
        result = reindex_posts_handle => {
            if let Err(e) = result {
                tracing::error!("Reindex posts consumer panicked: {:?}", e);
            }
        }
        result = reindex_users_handle => {
            if let Err(e) = result {
                tracing::error!("Reindex users consumer panicked: {:?}", e);
            }
        }
        result = delete_content_handle => {
            if let Err(e) = result {
                tracing::error!("Delete content consumer panicked: {:?}", e);
            }
        }
    }

    tracing::error!("Worker terminated unexpectedly");
    Ok(())
}
