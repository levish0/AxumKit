use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use entity::common::Role;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for user response.
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub handle: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_image: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<Role>,
    pub is_banned: bool,
    /// Whether the account has a password set (false for OAuth-only accounts)
    pub has_password: bool,
    pub created_at: DateTime<Utc>,
}

impl IntoResponse for UserResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
