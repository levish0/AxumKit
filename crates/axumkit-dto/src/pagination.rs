use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Common cursor direction for bidirectional pagination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
pub enum CursorDirection {
    /// Get older items (earlier in time, smaller UUID v7)
    Older,
    /// Get newer items (more recent, larger UUID v7)
    Newer,
}
