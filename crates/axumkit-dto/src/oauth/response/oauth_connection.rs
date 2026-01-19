use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axumkit_entity::common::OAuthProvider;
use axumkit_entity::user_oauth_connections::Model as OAuthConnectionModel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// OAuth 연결 정보 응답
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OAuthConnectionResponse {
    /// OAuth provider (Google, Github)
    pub provider: OAuthProvider,

    /// 연결 생성 시각
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

/// OAuth 연결 목록 응답
#[derive(Debug, Serialize, ToSchema)]
pub struct OAuthConnectionListResponse {
    pub connections: Vec<OAuthConnectionResponse>,
}

impl IntoResponse for OAuthConnectionListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
