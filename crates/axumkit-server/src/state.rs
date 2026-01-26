use redis::aio::ConnectionManager as RedisClient;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::connection::{MeilisearchClient, R2Client, SeaweedFsClient};
use axumkit_dto::action_logs::ActionLogResponse;
use reqwest::Client as HttpClient;
use sea_orm::DatabaseConnection as PostgresqlClient;

/// JetStream context for publishing jobs to NATS
pub type WorkerClient = Arc<async_nats::jetstream::Context>;

/// NATS client for Core Pub/Sub (EventStream)
pub type NatsClient = async_nats::Client;

/// Broadcast sender for EventStream SSE fan-out
pub type EventStreamSender = broadcast::Sender<ActionLogResponse>;

#[derive(Clone)]
pub struct AppState {
    pub write_db: PostgresqlClient,
    pub read_db: PostgresqlClient,
    pub r2_client: R2Client,
    pub seaweedfs_client: SeaweedFsClient,
    /// Redis for sessions, tokens, rate-limiting (persistent, AOF)
    pub redis_session: RedisClient,
    /// Redis for document cache (volatile, LRU eviction)
    pub redis_cache: RedisClient,
    /// NATS JetStream context for worker job queue
    pub worker: WorkerClient,
    /// NATS client for EventStream Pub/Sub
    pub nats_client: NatsClient,
    /// Broadcast channel for SSE event fan-out
    pub eventstream_tx: EventStreamSender,
    pub http_client: HttpClient,
    pub meilisearch_client: MeilisearchClient,
}
