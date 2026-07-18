use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Result of a board post moderation action, echoing the post's resulting state.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BoardPostModerationResponse {
    pub post_id: Uuid,
    pub is_pinned: bool,
    pub is_locked: bool,
}

impl IntoResponse for BoardPostModerationResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
