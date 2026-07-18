use axum::Json;
use axum::response::IntoResponse;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for check handle available response.
pub struct CheckHandleAvailableResponse {
    pub available: bool,
}

impl IntoResponse for CheckHandleAvailableResponse {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
