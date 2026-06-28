use crate::CacheClient;
use crate::nats::streams::dlq_subject;
use async_nats::jetstream::{
    Context as JetStream, consumer::pull::Config as PullConfig, message::AckKind,
};
use futures::StreamExt;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use std::future::Future;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Ack wait for in-flight messages. Long-running handlers are kept alive by
/// in-progress acks sent every `ACK_WAIT / 2`, so this only bounds how fast a
/// message is redelivered after a worker crash.
const ACK_WAIT: Duration = Duration::from_secs(30);

/// TTL for per-message dedup markers (24h) — long enough to outlive all retries/redeliveries.
const DEDUP_TTL_SECS: u64 = 24 * 60 * 60;

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
    /// When set, redelivered messages already processed once are skipped (by stream
    /// sequence). Enable only for non-idempotent handlers (e.g. email).
    dedup: Option<CacheClient>,
}

impl NatsConsumer {
    /// Helper function for new.
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
            dedup: None,
        }
    }

    /// Helper function for with backoff.
    pub fn with_backoff(mut self, backoff: Vec<Duration>) -> Self {
        self.backoff = backoff;
        self
    }

    /// Enable per-message dedup (skip already-processed redeliveries) using Redis.
    /// Use for non-idempotent handlers where reprocessing a redelivered message causes
    /// a visible duplicate (e.g. sending an email twice).
    pub fn with_dedup(mut self, cache_client: CacheClient) -> Self {
        self.dedup = Some(cache_client);
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
                    ack_wait: ACK_WAIT,
                    // Don't deliver more than we can process concurrently; otherwise
                    // messages buffered behind the semaphore burn their ack_wait
                    // while waiting and get redelivered as duplicates.
                    max_ack_pending: self.concurrency as i64,
                    ..Default::default()
                },
            )
            .await?;

        tracing::info!(
            consumer = %self.consumer_name,
            stream = %self.stream_name,
            concurrency = self.concurrency,
            max_deliver,
            "Consumer started"
        );

        let semaphore = Arc::new(Semaphore::new(self.concurrency));
        let mut messages = consumer.messages().await?;

        while let Some(msg_result) = messages.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!(error = %e, "Error receiving message");
                    continue;
                }
            };

            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break, // Semaphore closed, shutdown
            };

            let handler = handler.clone();
            let consumer_name = self.consumer_name.clone();
            let jetstream = self.jetstream.clone();
            let stream_name = self.stream_name.clone();
            let dedup = self.dedup.clone();

            tokio::spawn(async move {
                let _permit = permit;

                // Stream sequence is stable across redeliveries — used as the dedup identity.
                let seq = msg.info().ok().map(|info| info.stream_sequence);

                let job: T = match serde_json::from_slice(&msg.payload) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!(consumer = %consumer_name, error = %e, "Failed to deserialize job");
                        // Park the unparseable message in the DLQ, then terminate so it
                        // doesn't sit in the WorkQueue forever.
                        publish_to_dlq(
                            &jetstream,
                            &stream_name,
                            &consumer_name,
                            &msg.payload,
                            "deserialize_failed",
                        )
                        .await;
                        if let Err(e) = msg.ack_with(AckKind::Term).await {
                            tracing::error!(consumer = %consumer_name, error = %e, "Failed to terminate bad message");
                        }
                        return;
                    }
                };

                // Dedup: a redelivery of a message we already processed (ack lost after a
                // successful side effect) is skipped so non-idempotent handlers don't repeat it.
                if let (Some(redis), Some(seq)) = (dedup.as_ref(), seq)
                    && dedup_already_processed(redis, &consumer_name, seq).await
                {
                    tracing::info!(consumer = %consumer_name, seq, "Skipping already-processed message (dedup)");
                    if let Err(e) = msg.ack().await {
                        tracing::error!(consumer = %consumer_name, error = %e, "Failed to ack deduped message");
                    }
                    return;
                }

                // Run the handler while sending in-progress acks so handlers that
                // outlive ack_wait (reindex batches, large fan-outs) are not
                // redelivered mid-processing.
                let result = {
                    let mut handler_future = pin!(handler(job));
                    let mut heartbeat = tokio::time::interval(ACK_WAIT / 2);
                    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                    heartbeat.tick().await; // first tick fires immediately; skip it

                    loop {
                        tokio::select! {
                            result = &mut handler_future => break result,
                            _ = heartbeat.tick() => {
                                if let Err(e) = msg.ack_with(AckKind::Progress).await {
                                    tracing::warn!(consumer = %consumer_name, error = %e, "Failed to send in-progress ack");
                                }
                            }
                        }
                    }
                };

                match result {
                    Ok(()) => {
                        // Mark processed before acking so a redelivery (lost ack) is skipped
                        // by the dedup check above instead of reprocessing.
                        if let (Some(redis), Some(seq)) = (dedup.as_ref(), seq) {
                            dedup_mark_processed(redis, &consumer_name, seq).await;
                        }
                        if let Err(e) = msg.ack().await {
                            tracing::error!(consumer = %consumer_name, error = %e, "Failed to ack message");
                        }
                    }
                    Err(e) => {
                        let delivered = msg.info().map(|info| info.delivered).unwrap_or(0);
                        if delivered >= max_deliver {
                            // Final attempt: park in the DLQ for inspection/replay, then
                            // terminate so it doesn't sit in the WorkQueue stream forever.
                            tracing::error!(
                                consumer = %consumer_name,
                                error = %e,
                                delivered,
                                "Job failed permanently after max deliveries; dead-lettering message"
                            );
                            publish_to_dlq(
                                &jetstream,
                                &stream_name,
                                &consumer_name,
                                &msg.payload,
                                "max_deliveries_exhausted",
                            )
                            .await;
                            if let Err(e) = msg.ack_with(AckKind::Term).await {
                                tracing::error!(consumer = %consumer_name, error = %e, "Failed to terminate message");
                            }
                        } else {
                            tracing::warn!(consumer = %consumer_name, error = %e, delivered, "Job failed");
                            // Nak with None delay - NATS uses the backoff config automatically
                            if let Err(e) = msg.ack_with(AckKind::Nak(None)).await {
                                tracing::error!(consumer = %consumer_name, error = %e, "Failed to nak message");
                            }
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

/// Republish a failed/undeliverable message to the dead-letter stream for inspection/replay.
/// Best-effort: a DLQ publish failure is logged but never blocks terminating the original.
async fn publish_to_dlq(
    jetstream: &JetStream,
    stream_name: &str,
    consumer_name: &str,
    payload: &[u8],
    reason: &str,
) {
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("X-DLQ-Origin-Stream", stream_name);
    headers.insert("X-DLQ-Consumer", consumer_name);
    headers.insert("X-DLQ-Reason", reason);

    match jetstream
        .publish_with_headers(dlq_subject(stream_name), headers, payload.to_vec().into())
        .await
    {
        Ok(ack) => {
            if let Err(e) = ack.await {
                tracing::error!(consumer = %consumer_name, error = %e, "Failed to confirm DLQ publish");
            }
        }
        Err(e) => {
            tracing::error!(consumer = %consumer_name, reason, error = %e, "Failed to publish to DLQ");
        }
    }
}

fn dedup_key(consumer_name: &str, seq: u64) -> String {
    format!("worker_dedup:{consumer_name}:{seq}")
}

/// Returns whether this message sequence was already processed. Fail-open: if Redis is
/// unavailable, treat as not-processed (better a rare duplicate than skipping real work).
async fn dedup_already_processed(redis: &CacheClient, consumer_name: &str, seq: u64) -> bool {
    let mut conn = (**redis).clone();
    conn.exists::<_, bool>(dedup_key(consumer_name, seq))
        .await
        .unwrap_or(false)
}

async fn dedup_mark_processed(redis: &CacheClient, consumer_name: &str, seq: u64) {
    let mut conn = (**redis).clone();
    if let Err(e) = conn
        .set_ex::<_, _, ()>(dedup_key(consumer_name, seq), 1, DEDUP_TTL_SECS)
        .await
    {
        tracing::warn!(consumer = %consumer_name, seq, error = %e, "Failed to set dedup marker");
    }
}
