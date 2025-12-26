pub mod clients;
pub mod config;
pub mod connection;
pub mod jobs;
pub mod templates;
pub mod utils;

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