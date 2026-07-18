use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use super::BoardCommentResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct BoardCommentListResponse {
    pub data: Vec<BoardCommentResponse>,
    pub has_newer: bool,
    pub has_older: bool,
}

impl IntoResponse for BoardCommentListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
