use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// OAuth authorization URL response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OAuthUrlResponse {
    /// Google/GitHub OAuth authorization URL (includes state parameter)
    pub auth_url: String,
}

impl IntoResponse for OAuthUrlResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
