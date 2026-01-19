//! E2E test framework for AxumKit.
//!
//! Provides test infrastructure using Docker containers for PostgreSQL, Redis, MeiliSearch,
//! NATS, SeaweedFS, and the AxumKit server/worker.

pub mod fixtures;
pub mod helpers;

pub use fixtures::TestServer;
pub use helpers::TestUser;
