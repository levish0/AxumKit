//! Job payload types crossing the server → worker boundary.
//!
//! The module tree mirrors `worker::jobs` so the worker can re-export each
//! payload from its own handler module (`pub use job_queue::jobs::…`) and the
//! server can import the payload it enqueues. Only the wire types live here — the
//! handlers, consumers and side effects stay in the worker.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Transactional email jobs.
pub mod email {
    use super::*;

    /// Worker job payload for send email job.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SendEmailJob {
        pub to: String,
        pub subject: String,
        pub template: EmailTemplate,
    }

    /// Enum describing email template.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum EmailTemplate {
        Verification {
            username: String,
            token: String,
            valid_minutes: u64,
        },
        PasswordReset {
            handle: String,
            token: String,
            valid_minutes: u64,
        },
        EmailChange {
            username: String,
            token: String,
            valid_minutes: u64,
        },
        AccountDeletion {
            username: String,
            token: String,
            valid_minutes: u64,
        },
        DeviceVerification {
            username: String,
            device: String,
            token: String,
            valid_minutes: u64,
        },
        SecurityAlert {
            username: String,
            event: String,
        },
        Custom {
            html_content: String,
        },
    }
}

/// Search indexing jobs. Mirrors `worker::jobs::index`.
pub mod index {
    /// User index jobs.
    pub mod user {
        use super::super::*;

        /// Worker job payload for index user job.
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct IndexUserJob {
            pub user_id: Uuid,
            pub action: UserIndexAction,
        }

        /// Enum describing user index action.
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum UserIndexAction {
            Index,
            Delete,
        }
    }
}

/// OAuth profile image fetch jobs.
pub mod oauth {
    use super::*;

    /// Worker job payload for fetching an OAuth profile image.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct OAuthProfileImageJob {
        pub user_id: Uuid,
        pub image_url: String,
    }
}

/// Full-reindex jobs plus their pure constructors.
pub mod reindex {
    use super::*;

    /// Default number of items processed per reindex batch.
    pub const DEFAULT_BATCH_SIZE: u32 = 1_000;

    /// Common job fields for all reindex jobs
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ReindexJobBase {
        /// Cursor for pagination (None = start from beginning)
        pub after_id: Option<Uuid>,
        /// Number of items to process per batch (default: 1,000)
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

    /// Create a new [`ReindexUsersJob`] that starts from the beginning.
    pub fn create_reindex_users_job(reindex_id: Uuid, batch_size: Option<u32>) -> ReindexUsersJob {
        ReindexUsersJob {
            base: ReindexJobBase {
                after_id: None,
                batch_size: batch_size.unwrap_or(DEFAULT_BATCH_SIZE),
                reindex_id,
                batch_number: 1,
            },
        }
    }
}

// Aliases mirroring `worker::jobs`, so server-side imports read the same as the
// worker's own module names.
pub use index::user as user_index;
