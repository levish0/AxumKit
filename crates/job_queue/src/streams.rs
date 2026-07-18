//! JetStream stream creation for the shared job-queue contract.
//!
//! Both binaries call [`initialize_all_streams`] at startup: the worker before it
//! binds consumers, and the API server before it publishes. Creation is
//! idempotent (`get_or_create_stream`), so whichever process boots first declares
//! the streams — without this, a fresh NATS with the server up before any worker
//! made every publish fail with "no stream matches subject" (dropped index/notify
//! jobs and user-visible signup-email failures).

use crate::subjects::{DLQ_MAX_AGE, DLQ_STREAM, DLQ_SUBJECT_PREFIX, JOB_STREAMS};
use async_nats::jetstream::{
    Context as JetStream,
    context::CreateStreamError,
    stream::{Config as StreamConfig, RetentionPolicy},
};

/// Initialize all job streams with WorkQueue retention policy
pub async fn initialize_all_streams(jetstream: &JetStream) -> Result<(), CreateStreamError> {
    for (stream_name, subject) in JOB_STREAMS {
        jetstream
            .get_or_create_stream(StreamConfig {
                name: stream_name.to_string(),
                subjects: vec![subject.to_string()],
                retention: RetentionPolicy::WorkQueue,
                ..Default::default()
            })
            .await?;
        tracing::info!("Stream {} ready (subject: {})", stream_name, subject);
    }

    // Dead-letter stream captures terminated messages from every job stream. Uses Limits
    // retention (not WorkQueue) so messages persist until DLQ_MAX_AGE for inspection/replay.
    jetstream
        .get_or_create_stream(StreamConfig {
            name: DLQ_STREAM.to_string(),
            subjects: vec![format!("{DLQ_SUBJECT_PREFIX}>")],
            retention: RetentionPolicy::Limits,
            max_age: DLQ_MAX_AGE,
            ..Default::default()
        })
        .await?;
    tracing::info!("Stream {} ready (dead-letter)", DLQ_STREAM);

    Ok(())
}
