use anyhow::Result;
use config::WorkerConfig;
use futures::FutureExt;
use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{error, info};
use worker::clients;
use worker::connection;
use worker::jobs::{self, WorkerContext};
use worker::nats::shutdown;
use worker::nats::streams::initialize_all_streams;
use worker::utils;
use worker::{CacheClient, DbPool, LockClient};

const CONSUMER_RESTART_DELAY: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug)]
enum ConsumerKind {
    Email,
    IndexUser,
    ReindexUsers,
    OAuthProfileImage,
}

impl ConsumerKind {
    const ALL: [Self; 4] = [
        Self::Email,
        Self::IndexUser,
        Self::ReindexUsers,
        Self::OAuthProfileImage,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::IndexUser => "index_user",
            Self::ReindexUsers => "reindex_users",
            Self::OAuthProfileImage => "oauth_profile_image",
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

/// Resolves when the process receives a termination signal: SIGTERM (e.g.
/// `docker stop`) or SIGINT (Ctrl-C) on Unix, Ctrl-C on other platforms.
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

async fn run_consumer(kind: ConsumerKind, ctx: WorkerContext) -> Result<()> {
    match kind {
        ConsumerKind::Email => jobs::email::run_consumer(ctx).await,
        ConsumerKind::IndexUser => jobs::index::user::run_consumer(ctx).await,
        ConsumerKind::ReindexUsers => jobs::reindex::users::run_consumer(ctx).await,
        ConsumerKind::OAuthProfileImage => jobs::oauth::run_consumer(ctx).await,
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
    let db_conn = connection::establish_connection().await?;
    let db_pool: DbPool = Arc::new(db_conn);

    info!("Shared clients created");

    // Connect to Redis Cache (for view counts, etc.)
    let redis_cache_url = config.redis_cache_url();
    info!(url = %redis_cache_url, "Connecting to Redis Cache");
    let redis_cache_client = redis::Client::open(redis_cache_url)?;
    let redis_cache_conn = redis::aio::ConnectionManager::new(redis_cache_client).await?;
    let cache_client: CacheClient = Arc::new(redis_cache_conn);

    // Connect to Redis Lock (distributed locks; must be a noeviction instance)
    let redis_lock_url = config.redis_lock_url();
    info!(url = %redis_lock_url, "Connecting to Redis Lock");
    let redis_lock_client = redis::Client::open(redis_lock_url)?;
    let redis_lock_conn = redis::aio::ConnectionManager::new(redis_lock_client).await?;
    let lock_client: LockClient = Arc::new(redis_lock_conn);

    // Connect to R2 assets
    info!("Connecting to R2 assets...");
    let r2_assets = connection::establish_r2_assets_connection(config).await?;

    // Connect to NATS
    info!(url = %config.nats_url, "Connecting to NATS");
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
        lock_client: lock_client.clone(),
        r2_assets,
        jetstream,
        config,
    };

    info!("Starting job consumers...");

    // Spawn all job consumers with supervisor loop
    let mut consumers = JoinSet::new();
    for kind in ConsumerKind::ALL {
        spawn_consumer(&mut consumers, kind, ctx.clone());
    }

    info!("Job consumers started");

    // Start cron scheduler (tokio-cron-scheduler)
    info!("Starting cron scheduler...");
    let mut cron_scheduler = jobs::cron::start_scheduler(
        ctx.db_pool.clone(),
        lock_client,
        ctx.cache_client.clone(),
        ctx.r2_assets.clone(),
        config,
    )
    .await?;

    // Translate OS termination signals into the process-wide shutdown signal
    // that every consumer's pull loop watches.
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        info!("Termination signal received; beginning graceful shutdown");
        shutdown::trigger();
    });

    info!("All workers running");

    // Supervisor loop: restart a consumer that exits/panics unexpectedly, but
    // stop restarting once shutdown begins. `biased` checks shutdown first so a
    // consumer that returns from its own drain is not restarted mid-shutdown.
    loop {
        tokio::select! {
            biased;
            _ = shutdown::wait() => break,
            joined = consumers.join_next() => {
                let exited_consumer = match joined {
                    Some(Ok(exit)) => exit,
                    Some(Err(e)) => {
                        error!(error = ?e, "Failed to join consumer task");
                        continue;
                    }
                    None => break,
                };

                match exited_consumer.outcome {
                    ConsumerExitOutcome::Completed(Ok(())) => {
                        error!(consumer = exited_consumer.kind.name(), "Consumer exited unexpectedly");
                    }
                    ConsumerExitOutcome::Completed(Err(e)) => {
                        error!(consumer = exited_consumer.kind.name(), error = %e, "Consumer stopped with error");
                    }
                    ConsumerExitOutcome::Panicked(panic) => {
                        error!(consumer = exited_consumer.kind.name(), panic = %panic, "Consumer panicked");
                    }
                }

                tokio::time::sleep(CONSUMER_RESTART_DELAY).await;
                info!(
                    consumer = exited_consumer.kind.name(),
                    restart_delay_secs = CONSUMER_RESTART_DELAY.as_secs(),
                    "Restarting consumer"
                );
                spawn_consumer(&mut consumers, exited_consumer.kind, ctx.clone());
            }
        }
    }

    // Stop the cron scheduler so no new scheduled job starts (and strands its
    // distributed lock) during the drain window.
    if let Err(e) = cron_scheduler.shutdown().await {
        error!(error = %e, "Failed to shut down cron scheduler");
    }

    // Graceful shutdown: consumers have stopped pulling and are draining their
    // in-flight handlers. Wait for them to finish, bounded so a stuck handler
    // can't block the deploy forever.
    info!("Waiting for consumers to drain (up to 30s)...");
    match tokio::time::timeout(Duration::from_secs(30), async {
        while consumers.join_next().await.is_some() {}
    })
    .await
    {
        Ok(()) => info!("All consumers drained; worker shut down cleanly"),
        Err(_) => error!("Timed out draining consumers; exiting with in-flight work incomplete"),
    }

    Ok(())
}
