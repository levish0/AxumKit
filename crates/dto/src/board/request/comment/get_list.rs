use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::pagination::CursorDirection;

#[derive(Debug, Deserialize, ToSchema, Validate, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct GetBoardCommentsRequest {
    pub post_id: Uuid,
    /// `None` lists top-level comments; `Some(id)` lists the replies under that comment.
    pub parent_comment_id: Option<Uuid>,
    /// Cursor keyed by a comment id (UUIDv7, time-ordered). Omit to fetch the
    /// first (oldest) page from the start of the thread.
    pub cursor_id: Option<Uuid>,
    /// Deep-link anchor: return the page window *containing* this comment
    /// instead of paging from a cursor. The comment must belong to the listing
    /// scope (`post_id` + `parent_comment_id`). Mutually exclusive with
    /// `cursor_id`.
    pub focus_comment_id: Option<Uuid>,
    /// Pagination direction relative to the cursor. This list reads **oldest-first**,
    /// and every page is returned in that order regardless of direction —
    /// `Older`/`Newer` only choose which side of the cursor to read. Defaults to
    /// `Newer` (advance toward newer comments) when a cursor is set; ignored otherwise.
    pub cursor_direction: Option<CursorDirection>,
    #[validate(range(min = 1, max = 100, message = "Limit must be between 1 and 100."))]
    pub limit: u64,
}
