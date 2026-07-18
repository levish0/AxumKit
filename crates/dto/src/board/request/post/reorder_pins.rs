use crate::validator::string_validator::validate_not_blank;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Rewrites a board's pin display order.
///
/// `post_ids` is the board's whole pin list in the order it should render, top
/// first; the server derives each position from the list index, so no numbers
/// are sent. It must name exactly the board's current pin set — anything else is
/// a stale list and is rejected whole (409) rather than partially applied.
/// Reordering neither pins nor unpins.
///
/// Like every board moderation action, this is recorded in the moderation log
/// and so requires a reason.
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BoardPostReorderPinsRequest {
    pub board_id: Uuid,
    #[validate(length(min = 1, message = "Pin order must name at least one post."))]
    pub post_ids: Vec<Uuid>,
    #[validate(length(
        min = 1,
        max = 500,
        message = "Reason must be between 1 and 500 characters."
    ))]
    #[validate(custom(function = "validate_not_blank"))]
    pub reason: String,
}
