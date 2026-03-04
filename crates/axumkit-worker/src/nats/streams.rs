use async_nats::jetstream::{
    Context as JetStream,
    stream::{Config as StreamConfig, RetentionPolicy},
};

// Stream names
pub const EMAIL_STREAM: &str = "axumkit_jobs_email";
pub const INDEX_USER_STREAM: &str = "axumkit_jobs_index_user";
pub const REINDEX_USERS_STREAM: &str = "axumkit_jobs_reindex_users";

// Subjects (for publishing)
pub const EMAIL_SUBJECT: &str = "axumkit.jobs.email";
pub const INDEX_USER_SUBJECT: &str = "axumkit.jobs.index.user";
pub const REINDEX_USERS_SUBJECT: &str = "axumkit.jobs.reindex.users";

// Consumer names
pub const EMAIL_CONSUMER: &str = "email-consumer";
pub const INDEX_USER_CONSUMER: &str = "user-index-consumer";
pub const REINDEX_USERS_CONSUMER: &str = "reindex-users-consumer";

/// Stream and subject pairs for initialization
const STREAMS: &[(&str, &str)] = &[
    (EMAIL_STREAM, EMAIL_SUBJECT),
    (INDEX_USER_STREAM, INDEX_USER_SUBJECT),
    (REINDEX_USERS_STREAM, REINDEX_USERS_SUBJECT),
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
    Ok(())
}
