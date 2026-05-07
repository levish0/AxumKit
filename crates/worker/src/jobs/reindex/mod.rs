pub mod common;
pub mod users;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Common job fields for all reindex jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexJobBase {
    /// Cursor for pagination (None = start from beginning)
    pub after_id: Option<Uuid>,
    /// Number of items to process per batch (default: 10,000)
    pub batch_size: u32,
    /// Unique ID for this reindex operation (for logging)
    pub reindex_id: Uuid,
    /// Current batch number (for logging)
    pub batch_number: u32,
}

/// Job to reindex all users in batches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexUsersJob {
    #[serde(flatten)]
    pub base: ReindexJobBase,
}

pub use users::create_reindex_users_job;
