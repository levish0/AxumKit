use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// A group's granted permission codenames.
pub struct AclGroupPermissionsResponse {
    pub group_id: Uuid,
    /// Granted permission codenames (e.g. "board:pin_post")
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Every permission codename the application defines — what an admin UI can
/// offer as checkboxes (Django's permission list).
pub struct AclPermissionListResponse {
    pub permissions: Vec<String>,
}

impl IntoResponse for AclGroupPermissionsResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

impl IntoResponse for AclPermissionListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
