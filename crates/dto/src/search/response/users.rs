use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserSearchItem {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchUsersResponse {
    pub users: Vec<UserSearchItem>,
    pub page: u32,
    pub page_size: u32,
    pub total_hits: u64,
    pub total_pages: u32,
}

impl IntoResponse for SearchUsersResponse {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
