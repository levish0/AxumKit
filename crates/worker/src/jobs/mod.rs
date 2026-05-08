pub mod cron;
pub mod email;
pub mod index;
pub mod oauth;
pub mod reindex;

// Re-exports for backward compatibility with server
pub use index::user as user_index;

use crate::config::WorkerConfig;
use crate::connection::R2AssetsClient;
use crate::nats::JetStreamContext;
use crate::{CacheClient, DbPool, LockClient, Mailer, SearchClient};

/// Shared context for worker registration
#[derive(Clone)]
pub struct WorkerContext {
    pub mailer: Mailer,
    pub meili_client: SearchClient,
    pub db_pool: DbPool,
    pub cache_client: CacheClient,
    pub lock_client: LockClient,
    pub r2_assets: R2AssetsClient,
    pub jetstream: JetStreamContext,
    pub config: &'static WorkerConfig,
}
