use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use chrono::{DateTime, Utc};
use entity::common::Role;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response payload for grant role response.
pub struct GrantRoleResponse {
    pub user_id: Uuid,
    pub role: Role,
    pub expires_at: Option<DateTime<Utc>>,
}

impl IntoResponse for GrantRoleResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
