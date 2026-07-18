use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use entity::common::OAuthProvider;
use entity::user_oauth_connections::Model as OAuthConnectionModel;
use serde::Serialize;
use utoipa::ToSchema;

/// OAuth connection info response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[schema(description = "One OAuth provider currently linked to the user.")]
pub struct OAuthConnectionResponse {
    /// OAuth provider (Google, Github)
    pub provider: OAuthProvider,

    /// When the connection was created
    pub created_at: DateTime<Utc>,
}

impl From<OAuthConnectionModel> for OAuthConnectionResponse {
    fn from(model: OAuthConnectionModel) -> Self {
        Self {
            provider: model.provider,
            created_at: model.created_at,
        }
    }
}

/// OAuth connection list response
#[derive(Debug, Serialize, ToSchema)]
#[schema(description = "List of OAuth providers linked to the user.")]
pub struct OAuthConnectionListResponse {
    pub connections: Vec<OAuthConnectionResponse>,
}

impl IntoResponse for OAuthConnectionListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
