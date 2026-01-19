pub mod consumer;
pub mod publisher;
pub mod streams;

use std::sync::Arc;

/// Shared NATS client type
pub type NatsClient = Arc<async_nats::Client>;

/// Shared JetStream context type
pub type JetStreamContext = Arc<async_nats::jetstream::Context>;
