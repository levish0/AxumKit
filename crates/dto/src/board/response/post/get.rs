use crate::actor::ActorResponse;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BoardPostResponse {
    pub id: Uuid,
    pub board_id: Uuid,
    pub author_actor_id: Uuid,
    pub author: Option<ActorResponse>,
    pub title: String,
    /// Raw sevenmark markup (used to populate the edit form).
    pub content: String,
    pub is_pinned: bool,
    pub is_locked: bool,
    pub view_count: i32,
    pub comment_count: i32,
    /// Whether the caller can edit this post (author or moderator).
    pub can_edit: bool,
    /// Whether the caller can delete this post (author or moderator).
    pub can_delete: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl IntoResponse for BoardPostResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
