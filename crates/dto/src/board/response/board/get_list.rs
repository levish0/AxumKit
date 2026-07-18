use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use super::BoardResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct BoardListResponse {
    pub boards: Vec<BoardResponse>,
    pub current_page: u32,
    pub page_size: u32,
    pub has_more: bool,
}

impl IntoResponse for BoardListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
