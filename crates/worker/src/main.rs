use anyhow::Result;
use worker::clients;
use worker::config::WorkerConfig;
use worker::connection;
use worker::jobs::{self, WorkerContext};
use worker::nats::streams::initialize_all_streams;
use worker::utils;
use worker::{CacheClient, DbPool};
use futures::FutureExt;
use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{error, info};

const CONSUMER_RESTART_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy)]
enum ConsumerKind {
    Email,
    IndexUser,
    ReindexUsers,
}

impl ConsumerKind {
    const ALL: [ConsumerKind; 3] = [
        ConsumerKind::Email,
        ConsumerKind::IndexUser,
        ConsumerKind::ReindexUsers,
    ];

    fn name(self) -> &'static str {
        match self {
            ConsumerKind::Email => "email",
            ConsumerKind::IndexUser => "index_user",
            ConsumerKind::ReindexUsers => "reindex_users",
        }
    }
}

enum ConsumerExitOutcome {
    Completed(Result<()>),
    Panicked(String),
}

struct ConsumerExit {
    kind: ConsumerKind,
    outcome: ConsumerExitOutcome,
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

async fn run_consumer(kind: ConsumerKind, ctx: WorkerContext) -> Result<()> {
    match kind {
        ConsumerKind::Email => jobs::email::run_consumer(ctx).await,
        ConsumerKind::IndexUser => jobs::index::user::run_consumer(ctx).await,
        ConsumerKind::ReindexUsers => jobs::reindex::users::run_consumer(ctx).await,
    }
}

fn spawn_consumer(consumers: &mut JoinSet<ConsumerExit>, kind: ConsumerKind, ctx: WorkerContext) {
    consumers.spawn(async move {
        let result = AssertUnwindSafe(run_consumer(kind, ctx))
            .catch_unwind()
            .await;

        let outcome = match result {
            Ok(result) => ConsumerExitOutcome::Completed(result),
            Err(panic_payload) => ConsumerExitOutcome::Panicked(panic_message(panic_payload)),
        };

        ConsumerExit { kind, outcome }
    });

    info!(consumer = kind.name(), "Consumer task started");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize config (loads .env internally)
    let config = WorkerConfig::get();

    // Initialize logging
    utils::logger::init_tracing();

    info!("Starting worker...");

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
        r2_client,
        jetstream,
        config,
    };

    info!("Starting job consumers...");

    // Spawn all job consumers with supervisor pattern
    let mut consumers = JoinSet::new();
    for kind in ConsumerKind::ALL {
        spawn_consumer(&mut consumers, kind, ctx.clone());
    }

    // Start cron scheduler (tokio-cron-scheduler)
    info!("Starting cron scheduler...");
    let _cron_scheduler = jobs::cron::start_scheduler(
        ctx.db_pool.clone(),
        ctx.cache_client.clone(),
        ctx.r2_client.clone(),
        config,
    )
    .await?;

    info!("All workers running");

    // Supervisor loop: automatically restart crashed consumers
    loop {
        match consumers.join_next().await {
            Some(Ok(exit)) => {
                let name = exit.kind.name();

                match exit.outcome {
                    ConsumerExitOutcome::Completed(Ok(())) => {
                        error!(
                            consumer = name,
                            "Consumer exited unexpectedly without error"
                        );
                    }
                    ConsumerExitOutcome::Completed(Err(e)) => {
                        error!(consumer = name, error = %e, "Consumer exited with error");
                    }
                    ConsumerExitOutcome::Panicked(msg) => {
                        error!(consumer = name, panic = %msg, "Consumer panicked");
                    }
                }

                info!(
                    consumer = name,
                    delay_secs = CONSUMER_RESTART_DELAY.as_secs(),
                    "Restarting consumer"
                );
                tokio::time::sleep(CONSUMER_RESTART_DELAY).await;
                spawn_consumer(&mut consumers, exit.kind, ctx.clone());
            }
            Some(Err(e)) => {
                error!("JoinSet error: {:?}", e);
            }
            None => {
                error!("All consumers exited — this should never happen");
                break;
            }
        }
    }

    Ok(())
}
