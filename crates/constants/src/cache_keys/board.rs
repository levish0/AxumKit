//! Board-related Redis cache keys (view-count dedup + buffered deltas)

/// Per-viewer view dedup window TTL in seconds (6 hours).
/// A given viewer (account or IP) counts at most once per post per window.
pub const BOARD_POST_VIEW_DEDUP_TTL_SECONDS: u64 = 6 * 60 * 60;

/// View dedup key prefix.
/// Format: "board:post:view:{post_id}:{viewer}" where viewer is "u:{user_id}"
/// for authenticated callers or "ip:{ip}" for anonymous ones.
pub const BOARD_POST_VIEW_DEDUP_PREFIX: &str = "board:post:view:";

/// Buffered view-count deltas hash (field = post_id, value = un-flushed count).
/// Written by the server on each counted view, drained periodically by the
/// worker and added to `board_posts.view_count`.
pub const BOARD_POST_VIEW_PENDING_KEY: &str = "board:post:view:pending";

/// Build the per-viewer view dedup key.
pub fn board_post_view_dedup_key(post_id: &str, viewer: &str) -> String {
    format!("{}{}:{}", BOARD_POST_VIEW_DEDUP_PREFIX, post_id, viewer)
}
