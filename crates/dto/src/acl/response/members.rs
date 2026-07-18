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
/// Response payload for one ACL group membership.
pub struct AclGroupMemberResponse {
    pub id: Uuid,
    pub group_id: Uuid,
    /// User member (None for IP members)
    pub user_id: Option<Uuid>,
    /// IP/CIDR member (None for user members)
    #[schema(example = "192.168.1.0/24")]
    pub ip_address: Option<String>,
    pub reason: Option<String>,
    /// Membership expiration time (None = permanent)
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl IntoResponse for AclGroupMemberResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
/// Response payload for listing an ACL group's active members.
pub struct AclGroupMemberListResponse {
    pub data: Vec<AclGroupMemberResponse>,
    /// Whether older entries exist beyond this page.
    pub has_more: bool,
}

impl IntoResponse for AclGroupMemberListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
