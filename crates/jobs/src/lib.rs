//! PostgreSQL job registration and execution. Business payloads belong to their callers.
mod types;
pub use types::{AttemptOutcome, JobState};
pub mod entities;
mod postgres;
mod runtime;

pub use postgres::{Claim, JobError, PostgresQueue, TraceContext, enqueue};
pub use runtime::{HandlerFailure, WorkerSettings, run, run_one};
