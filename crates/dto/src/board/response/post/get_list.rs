use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use super::BoardPostResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct BoardPostListResponse {
    /// The board's pinned posts in display order, top first. Pins sit outside
    /// pagination: this is the complete set on every page, and none of them
    /// appear in `posts`.
    pub pinned: Vec<BoardPostResponse>,
    /// One page of the board's unpinned posts, newest first.
    pub posts: Vec<BoardPostResponse>,
    pub current_page: u32,
    pub page_size: u32,
    /// Whether another page of `posts` exists. Pins are not paged and never
    /// affect this.
    pub has_more: bool,
}

impl IntoResponse for BoardPostListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
