use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Active session entry for the authenticated user.")]
pub struct SessionInfo {
    /// Public session management identifier.
    pub management_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub max_expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    pub is_current: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "List of active sessions for the authenticated user.")]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionInfo>,
}

impl IntoResponse for ListSessionsResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
