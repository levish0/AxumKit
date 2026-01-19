use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct PublicUserProfile {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_image: Option<String>,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl IntoResponse for PublicUserProfile {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
