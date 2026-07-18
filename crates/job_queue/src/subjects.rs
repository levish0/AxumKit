//! JetStream stream names, publish subjects, durable consumer names and
//! dead-letter routing. These are the shared naming contract: the server
//! publishes to the `*_SUBJECT` names and the worker binds streams/consumers to
//! them. Stream *creation* (which needs the NATS client) lives in [`crate::streams`].

use std::time::Duration;

// Stream names
/// JetStream stream name for email jobs.
pub const EMAIL_STREAM: &str = "axumkit_jobs_email";
/// JetStream stream name for index user jobs.
pub const INDEX_USER_STREAM: &str = "axumkit_jobs_index_user";
/// JetStream stream name for reindex users jobs.
pub const REINDEX_USERS_STREAM: &str = "axumkit_jobs_reindex_users";
/// JetStream stream name for OAuth profile image jobs.
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
/// NATS subject used to publish email jobs.
pub const EMAIL_SUBJECT: &str = "axumkit.jobs.email";
/// NATS subject used to publish index user jobs.
pub const INDEX_USER_SUBJECT: &str = "axumkit.jobs.index.user";
/// NATS subject used to publish reindex users jobs.
pub const REINDEX_USERS_SUBJECT: &str = "axumkit.jobs.reindex.users";
/// NATS subject used to publish OAuth profile image jobs.
pub const OAUTH_PROFILE_IMAGE_SUBJECT: &str = "axumkit.jobs.oauth.profile_image";

/// Core NATS (non-JetStream) subject for realtime SSE fan-out events. Published
/// by the API server's eventstream publisher and consumed by its subscriber;
/// lives here so every NATS subject has one home.
pub const REALTIME_EVENTS_SUBJECT: &str = "axumkit.realtime.events";

// Consumer names
/// Durable consumer name for email jobs.
pub const EMAIL_CONSUMER: &str = "email-consumer";
/// Durable consumer name for index user jobs.
pub const INDEX_USER_CONSUMER: &str = "user-index-consumer";
/// Durable consumer name for reindex users jobs.
pub const REINDEX_USERS_CONSUMER: &str = "reindex-users-consumer";
/// Durable consumer name for OAuth profile image jobs.
pub const OAUTH_PROFILE_IMAGE_CONSUMER: &str = "oauth-profile-image-consumer";

/// Stream and subject pairs, one per work-queue job stream. The worker uses this
/// to create every stream on startup; keeping it beside the names means a new job
/// stream is declared in exactly one place.
pub const JOB_STREAMS: &[(&str, &str)] = &[
    (EMAIL_STREAM, EMAIL_SUBJECT),
    (INDEX_USER_STREAM, INDEX_USER_SUBJECT),
    (REINDEX_USERS_STREAM, REINDEX_USERS_SUBJECT),
    (OAUTH_PROFILE_IMAGE_STREAM, OAUTH_PROFILE_IMAGE_SUBJECT),
];
