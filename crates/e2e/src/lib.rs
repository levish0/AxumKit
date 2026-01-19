//! E2E test framework for V7 server.
//!
//! Provides test infrastructure using Docker containers for PostgreSQL, Redis, MeiliSearch,
//! NATS, SeaweedFS, Parser, and the V7 server/worker.

pub mod fixtures;
pub mod helpers;

pub use fixtures::TestServer;
pub use helpers::TestUser;
