//! Shared job-queue contract between the API server (producer) and the worker
//! (consumer).
//!
//! This crate holds the wire types that cross the NATS/JetStream boundary — job
//! payload structs, the stream/subject/consumer names, and the idempotent stream
//! declaration both binaries run at startup. It deliberately carries nothing from
//! the worker's execution stack (no database, mailer or search client) so the
//! server can enqueue jobs without linking it; the NATS client is the one runtime
//! dependency, inherent to declaring the streams.
//!
//! - [`jobs`] mirrors the worker's job module tree; each module exposes only the
//!   serializable payload for that job (plus a few pure constructors).
//! - [`subjects`] holds the JetStream stream names, publish subjects, durable
//!   consumer names and the dead-letter routing helpers.
//! - [`streams`] creates the streams (`get_or_create`, safe from both binaries).

pub mod jobs;
pub mod streams;
pub mod subjects;
