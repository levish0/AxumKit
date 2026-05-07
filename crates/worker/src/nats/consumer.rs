use async_nats::jetstream::{
    Context as JetStream, consumer::pull::Config as PullConfig, message::AckKind,
};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Default exponential backoff durations for retries
/// 1s, 2s, 4s, 8s, 16s (5 retries total)
fn default_backoff() -> Vec<Duration> {
    vec![
        Duration::from_secs(1),
        Duration::from_secs(2),
        Duration::from_secs(4),
        Duration::from_secs(8),
        Duration::from_secs(16),
    ]
}

/// Generic NATS consumer for job processing
pub struct NatsConsumer {
    jetstream: Arc<JetStream>,
    stream_name: String,
    consumer_name: String,
    concurrency: usize,
    backoff: Vec<Duration>,
}

impl NatsConsumer {
    pub fn new(
        jetstream: Arc<JetStream>,
        stream_name: &str,
        consumer_name: &str,
        concurrency: usize,
    ) -> Self {
        Self {
            jetstream,
            stream_name: stream_name.to_string(),
            consumer_name: consumer_name.to_string(),
            concurrency,
            backoff: default_backoff(),
        }
    }

    pub fn with_backoff(mut self, backoff: Vec<Duration>) -> Self {
        self.backoff = backoff;
        self
    }

    /// Run the consumer with the given handler function
    pub async fn run<T, F, Fut>(self, handler: F) -> anyhow::Result<()>
    where
        T: DeserializeOwned + Send + 'static,
        F: Fn(T) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Result<(), anyhow::Error>> + Send,
    {
        let stream = self.jetstream.get_stream(&self.stream_name).await?;

        let max_deliver = (self.backoff.len() + 1) as i64; // initial + retries

        let consumer = stream
            .get_or_create_consumer(
                &self.consumer_name,
                PullConfig {
                    durable_name: Some(self.consumer_name.clone()),
                    max_deliver,
                    backoff: self.backoff,
                    ack_wait: Duration::from_secs(30),
                    ..Default::default()
                },
            )
            .await?;

        tracing::info!(
            "Consumer {} started for stream {} (concurrency: {}, max_deliver: {})",
            self.consumer_name,
            self.stream_name,
            self.concurrency,
            max_deliver
        );

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut messages = consumer.messages().await?;

        while let Some(msg_result) = messages.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Error receiving message: {}", e);
                    continue;
                }
            };

            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break, // Semaphore closed, shutdown
            };

            let handler = handler.clone();
            let consumer_name = self.consumer_name.clone();

            tokio::spawn(async move {
                let _permit = permit;

                let job: T = match serde_json::from_slice(&msg.payload) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!("[{}] Failed to deserialize job: {}", consumer_name, e);
                        // Acknowledge bad messages to prevent infinite retry
                        if let Err(e) = msg.ack().await {
                            tracing::error!("[{}] Failed to ack bad message: {}", consumer_name, e);
                        }
                        return;
                    }
                };

                match handler(job).await {
                    Ok(()) => {
                        if let Err(e) = msg.ack().await {
                            tracing::error!("[{}] Failed to ack message: {}", consumer_name, e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[{}] Job failed: {}", consumer_name, e);
                        // Nak with None delay - NATS uses the backoff config automatically
                        if let Err(e) = msg.ack_with(AckKind::Nak(None)).await {
                            tracing::error!("[{}] Failed to nak message: {}", consumer_name, e);
                        }
                    }
                }
            });
        }

        Ok(())
    }
}
