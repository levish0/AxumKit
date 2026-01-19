pub mod clients;
pub mod config;
pub mod connection;
pub mod jobs;
pub mod nats;
pub mod templates;
pub mod utils;

use connection::{R2Client, SeaweedFsClient};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use meilisearch_sdk::client::Client as MeiliClient;
use redis::aio::ConnectionManager as RedisConnectionManager;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// Shared mailer for email sending
pub type Mailer = Arc<AsyncSmtpTransport<Tokio1Executor>>;

/// Shared MeiliSearch client
pub type SearchClient = Arc<MeiliClient>;

/// Shared database connection pool
pub type DbPool = Arc<DatabaseConnection>;

/// Shared Redis cache client (for view counts, etc.)
pub type CacheClient = Arc<RedisConnectionManager>;

/// Shared SeaweedFS storage client (for revision content)
pub type StorageClient = Arc<SeaweedFsClient>;

/// Shared R2 storage client (for sitemap files)
pub type R2StorageClient = Arc<R2Client>;
