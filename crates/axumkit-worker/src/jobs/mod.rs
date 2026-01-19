pub mod cron;
pub mod email;
pub mod index;
pub mod reindex;
pub mod storage;

// Re-exports for backward compatibility with axumkit_server
pub use index::post as post_index;
pub use index::user as user_index;

use crate::config::WorkerConfig;
use crate::connection::R2Client;
use crate::nats::JetStreamContext;
use crate::{CacheClient, DbPool, Mailer, SearchClient, StorageClient};

/// Shared context for worker registration
#[derive(Clone)]
pub struct WorkerContext {
    pub mailer: Mailer,
    pub meili_client: SearchClient,
    pub db_pool: DbPool,
    pub cache_client: CacheClient,
    pub storage_client: StorageClient,
    pub r2_client: R2Client,
    pub jetstream: JetStreamContext,
    pub config: &'static WorkerConfig,
}
