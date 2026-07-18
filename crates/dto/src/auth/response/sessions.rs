use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

/// Information about a single active session
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "Active session entry for the authenticated user.")]
pub struct SessionInfo {
    /// Identifier used for session management
    pub management_id: Uuid,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Current sliding expiration time
    pub expires_at: DateTime<Utc>,
    /// Absolute maximum expiration time
    pub max_expires_at: DateTime<Utc>,
    /// User-Agent at login time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// IP address at login time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// True if this is the session used by the current request
    pub is_current: bool,
}

/// Active session list response
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
