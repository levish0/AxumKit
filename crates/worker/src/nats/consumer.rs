use crate::CacheClient;
use crate::nats::shutdown;
use crate::nats::streams::dlq_subject;
use async_nats::jetstream::{
    Context as JetStream, consumer::pull::Config as PullConfig, message::AckKind,
};
use futures::{FutureExt, StreamExt};
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use std::any::Any;
use std::future::Future;
use std::panic::AssertUnwindSafe;
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

/// Backstop bound on a single handler run. The in-progress-ack heartbeat keeps a
/// running handler alive indefinitely, so without this a hung external call
/// would pin the message forever and silently leak the consumer's concurrency
/// slot — halting concurrency-1 queues outright. On expiry the handler is treated
/// as failed and nak'd, so it retries and eventually dead-letters.
///
/// This is a *backstop*: individual external calls are bounded by their own
/// client timeouts (R2 via `TimeoutConfig`), so it fires only for a handler that
/// hangs some other way or accumulates many slow calls. It is therefore set
/// generously so it never kills legitimately slow work; reindex consumers raise
/// it further via `with_handler_timeout` for their long batch runs.
const DEFAULT_HANDLER_TIMEOUT: Duration = Duration::from_secs(300);

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
    /// Upper bound on a single handler run; expiry nak's the message.
    handler_timeout: Duration,
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
            handler_timeout: DEFAULT_HANDLER_TIMEOUT,
        }
    }

    /// Helper function for with backoff.
    pub fn with_backoff(mut self, backoff: Vec<Duration>) -> Self {
        self.backoff = backoff;
        self
    }

    /// Override the per-handler timeout (default 2 min). Reindex consumers use a
    /// larger bound for their long batch runs.
    pub fn with_handler_timeout(mut self, handler_timeout: Duration) -> Self {
        self.handler_timeout = handler_timeout;
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

        // `create_consumer` (NATS create-or-update semantics) rather than
        // `get_or_create_consumer`: the latter returns an existing durable
        // consumer AS-IS, so tuning changes here (backoff, max_deliver,
        // ack_wait, max_ack_pending) would silently never reach a consumer
        // created by an earlier deploy. Create-or-update applies the config on
        // every startup; all fields we set are server-updatable.
        let consumer = stream
            .create_consumer(PullConfig {
                durable_name: Some(self.consumer_name.clone()),
                max_deliver,
                backoff: self.backoff,
                ack_wait: ACK_WAIT,
                // Don't deliver more than we can process concurrently; otherwise
                // messages buffered behind the semaphore burn their ack_wait
                // while waiting and get redelivered as duplicates.
                max_ack_pending: self.concurrency as i64,
                ..Default::default()
            })
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

        loop {
            // Stop pulling new messages on shutdown; in-flight handlers below are
            // drained after the loop. `biased` checks shutdown first each turn.
            let msg_result = tokio::select! {
                biased;
                _ = shutdown::wait() => {
                    tracing::info!(consumer = %self.consumer_name, "Shutdown requested; stopping message pull");
                    break;
                }
                next = messages.next() => match next {
                    Some(m) => m,
                    None => break,
                },
            };

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
            let handler_timeout = self.handler_timeout;

            tokio::spawn(async move {
                let _permit = permit;

                // Stream sequence is stable across redeliveries — used as the dedup identity.
                let seq = msg.info().ok().map(|info| info.stream_sequence);

                let job: T = match serde_json::from_slice(&msg.payload) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!(consumer = %consumer_name, error = %e, "Failed to deserialize job");
                        // Park the unparseable message in the DLQ, then terminate so it
                        // doesn't sit in the WorkQueue forever. Only terminate once the
                        // DLQ copy is confirmed; otherwise nak so the message is retried
                        // rather than lost with no copy anywhere.
                        let dlq_ok = publish_to_dlq(
                            &jetstream,
                            &stream_name,
                            &consumer_name,
                            &msg.payload,
                            "deserialize_failed",
                        )
                        .await;
                        if dlq_ok {
                            if let Err(e) = msg.ack_with(AckKind::Term).await {
                                tracing::error!(consumer = %consumer_name, error = %e, "Failed to terminate bad message");
                            }
                        } else if let Err(e) = msg.ack_with(AckKind::Nak(None)).await {
                            tracing::error!(consumer = %consumer_name, error = %e, "Failed to nak bad message after DLQ publish failure");
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
                // outlive ack_wait (reindex batches, large notification fan-outs)
                // are not redelivered mid-processing. The whole run is bounded by
                // `handler_timeout` so a hung external call can't pin the message
                // (and its concurrency slot) forever; expiry is treated as failure.
                let run_handler = async {
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

                // Catch a handler panic so it flows through the failure path
                // (nak / dead-letter) instead of unwinding the spawned task with
                // the message left unacked — which would wedge the queue: the
                // message redelivers, panics again, and never terminates.
                let result = match tokio::time::timeout(
                    handler_timeout,
                    AssertUnwindSafe(run_handler).catch_unwind(),
                )
                .await
                {
                    Ok(Ok(result)) => result,
                    Ok(Err(panic)) => Err(anyhow::anyhow!(
                        "handler panicked: {}",
                        panic_message(panic)
                    )),
                    Err(_) => Err(anyhow::anyhow!(
                        "handler timed out after {handler_timeout:?}"
                    )),
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
                            let dlq_ok = publish_to_dlq(
                                &jetstream,
                                &stream_name,
                                &consumer_name,
                                &msg.payload,
                                "max_deliveries_exhausted",
                            )
                            .await;
                            // Only drop the message once it is safely in the DLQ;
                            // otherwise nak so a later delivery can retry the DLQ
                            // publish rather than losing the job entirely.
                            if dlq_ok {
                                if let Err(e) = msg.ack_with(AckKind::Term).await {
                                    tracing::error!(consumer = %consumer_name, error = %e, "Failed to terminate message");
                                }
                            } else if let Err(e) = msg.ack_with(AckKind::Nak(None)).await {
                                tracing::error!(consumer = %consumer_name, error = %e, "Failed to nak message after DLQ publish failure");
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

        // Graceful drain: the pull loop has stopped, so wait for the in-flight
        // handlers (which each hold a semaphore permit) to finish before
        // returning. Acquiring every permit succeeds only once all handlers have
        // released theirs.
        tracing::info!(consumer = %self.consumer_name, "Draining in-flight handlers before shutdown");
        let _drained = semaphore.acquire_many(self.concurrency as u32).await;
        tracing::info!(consumer = %self.consumer_name, "Consumer drained");

        Ok(())
    }
}

/// Republish a failed/undeliverable message to the dead-letter stream for
/// inspection/replay. Returns whether the DLQ publish was confirmed by the
/// server — the caller must only `Term` (drop) the original when this is `true`,
/// otherwise the message would be lost with no copy anywhere.
async fn publish_to_dlq(
    jetstream: &JetStream,
    stream_name: &str,
    consumer_name: &str,
    payload: &[u8],
    reason: &str,
) -> bool {
    let mut headers = async_nats::HeaderMap::new();
    headers.insert("X-DLQ-Origin-Stream", stream_name);
    headers.insert("X-DLQ-Consumer", consumer_name);
    headers.insert("X-DLQ-Reason", reason);

    match jetstream
        .publish_with_headers(dlq_subject(stream_name), headers, payload.to_vec().into())
        .await
    {
        Ok(ack) => match ack.await {
            Ok(_) => true,
            Err(e) => {
                tracing::error!(consumer = %consumer_name, error = %e, "Failed to confirm DLQ publish");
                false
            }
        },
        Err(e) => {
            tracing::error!(consumer = %consumer_name, reason, error = %e, "Failed to publish to DLQ");
            false
        }
    }
}

/// Extracts a readable message from a caught panic payload.
fn panic_message(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
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
