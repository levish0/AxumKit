use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use axumkit_entity::common::Role;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response payload for revoke role response.
pub struct RevokeRoleResponse {
    pub user_id: Uuid,
    pub role: Role,
}

impl IntoResponse for RevokeRoleResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
