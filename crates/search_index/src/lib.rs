//! Shared Meilisearch index contract between the API server (reader) and the
//! worker (writer).
//!
//! The worker serializes each `Search*` struct into its index and the server
//! deserializes the same struct out — so the field names, index names and any
//! enum string encodings are defined exactly once here instead of being
//! mirrored (and silently drifting) across the two crates.

pub mod users;
