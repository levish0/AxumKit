pub mod common;
pub mod users;

// Reindex job payloads and their pure constructors live in the shared job_queue
// contract; the batch-processing consumers below are worker-only.
pub use job_queue::jobs::reindex::{ReindexJobBase, ReindexUsersJob, create_reindex_users_job};
