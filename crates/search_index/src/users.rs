//! `users` index contract.

use serde::{Deserialize, Serialize};

/// Meilisearch index uid for users.
pub const USERS_INDEX: &str = "users";

/// A user as stored in the `users` search index. The worker serializes this
/// from `users`; the server deserializes it for search (role/ban state is
/// enriched from the DB, not the index).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchUser {
    pub id: String,
    pub handle: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub profile_image: Option<String>,
}
