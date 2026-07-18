use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Result of a pin reorder, echoing the board's pin list in its new order.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BoardPostReorderPinsResponse {
    pub board_id: Uuid,
    pub post_ids: Vec<Uuid>,
}

impl IntoResponse for BoardPostReorderPinsResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
