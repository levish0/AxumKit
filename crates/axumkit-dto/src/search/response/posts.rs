use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct PostSearchHit {
    pub id: Uuid,
    pub author_id: Uuid,
    pub title: String,
    pub content_snippet: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchPostsResponse {
    pub hits: Vec<PostSearchHit>,
    pub page: u32,
    pub page_size: u32,
    pub total_hits: u64,
    pub total_pages: u32,
}

impl IntoResponse for SearchPostsResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
