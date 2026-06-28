use async_nats::jetstream::{
    Context as JetStream,
    stream::{Config as StreamConfig, RetentionPolicy},
};
use std::time::Duration;

// Stream names
pub const EMAIL_STREAM: &str = "axumkit_jobs_email";
pub const INDEX_USER_STREAM: &str = "axumkit_jobs_index_user";
pub const REINDEX_USERS_STREAM: &str = "axumkit_jobs_reindex_users";
pub const OAUTH_PROFILE_IMAGE_STREAM: &str = "axumkit_jobs_oauth_profile_image";

/// Dead-letter stream: messages that fail permanently (bad payload or max deliveries)
/// are republished here for inspection/replay instead of being dropped.
pub const DLQ_STREAM: &str = "axumkit_jobs_dlq";
/// Subject prefix for dead-lettered messages: `axumkit.dlq.{origin_stream}`.
pub const DLQ_SUBJECT_PREFIX: &str = "axumkit.dlq.";
/// Dead-letter retention (14 days).
pub const DLQ_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// Build the dead-letter subject for an origin stream.
pub fn dlq_subject(origin_stream: &str) -> String {
    format!("{DLQ_SUBJECT_PREFIX}{origin_stream}")
}

// Subjects (for publishing)
pub const EMAIL_SUBJECT: &str = "axumkit.jobs.email";
pub const INDEX_USER_SUBJECT: &str = "axumkit.jobs.index.user";
pub const REINDEX_USERS_SUBJECT: &str = "axumkit.jobs.reindex.users";
pub const OAUTH_PROFILE_IMAGE_SUBJECT: &str = "axumkit.jobs.oauth.profile_image";

// Consumer names
pub const EMAIL_CONSUMER: &str = "email-consumer";
pub const INDEX_USER_CONSUMER: &str = "user-index-consumer";
pub const REINDEX_USERS_CONSUMER: &str = "reindex-users-consumer";
pub const OAUTH_PROFILE_IMAGE_CONSUMER: &str = "oauth-profile-image-consumer";

/// Stream and subject pairs for initialization
const STREAMS: &[(&str, &str)] = &[
    (EMAIL_STREAM, EMAIL_SUBJECT),
    (INDEX_USER_STREAM, INDEX_USER_SUBJECT),
    (REINDEX_USERS_STREAM, REINDEX_USERS_SUBJECT),
    (OAUTH_PROFILE_IMAGE_STREAM, OAUTH_PROFILE_IMAGE_SUBJECT),
];

/// Initialize all job streams with WorkQueue retention policy
pub async fn initialize_all_streams(jetstream: &JetStream) -> anyhow::Result<()> {
    for (stream_name, subject) in STREAMS {
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
