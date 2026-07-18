use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response payload for one ACL group.
pub struct AclGroupResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    /// System groups are known to the code and cannot be deleted.
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

impl IntoResponse for AclGroupResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response payload for listing ACL groups.
pub struct AclGroupListResponse {
    pub groups: Vec<AclGroupResponse>,
}

impl IntoResponse for AclGroupListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
