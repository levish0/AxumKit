use axum::Json;
use axum::response::IntoResponse;
use serde::Serialize;
use utoipa::ToSchema;

use super::ActionLogResponse;

#[derive(Debug, Serialize, ToSchema)]
pub struct ActionLogListResponse {
    pub data: Vec<ActionLogResponse>,
    /// Whether there are newer (more recent) action logs
    pub has_newer: bool,
    /// Whether there are older action logs
    pub has_older: bool,
}

impl IntoResponse for ActionLogListResponse {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
