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
/// Response payload for public user profile.
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
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub roles: Vec<Role>,
    /// True if the user is deactivated (soft-deleted); handle/display_name stay exposed while the profile is masked.
    pub deactivated: bool,
    /// True if the user currently has an active (non-expired) ban.
    pub is_banned: bool,
    /// Ban expiry time. `None` means either not banned or a permanent ban.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banned_until: Option<DateTime<Utc>>,
    /// Reason recorded for the active ban, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ban_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl IntoResponse for PublicUserProfile {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
