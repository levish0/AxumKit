use crate::actor::ActorResponse;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BoardCommentResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    /// `None` for a top-level comment; the thread root's id for a reply.
    pub parent_comment_id: Option<Uuid>,
    pub author_actor_id: Uuid,
    pub author: Option<ActorResponse>,
    /// Raw sevenmark markup (used to populate the edit form).
    pub content: String,
    pub reply_count: i32,
    /// Whether the caller can edit this comment (author or moderator).
    pub can_edit: bool,
    /// Whether the caller can delete this comment (author or moderator).
    pub can_delete: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl IntoResponse for BoardCommentResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
