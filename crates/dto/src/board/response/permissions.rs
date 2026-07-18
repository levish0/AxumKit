use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Board-level capability flags for the caller, mirroring the `BoardPermission` rules.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BoardPermissionsResponse {
    pub can_view: bool,
    pub can_write: bool,
    pub can_moderate: bool,
    pub can_manage: bool,
}

impl IntoResponse for BoardPermissionsResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
