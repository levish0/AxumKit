use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

/// Google One Tap nonce response
#[derive(Debug, Clone, Serialize, ToSchema)]
#[schema(description = "Single-use nonce to pass to Google One Tap initialization.")]
pub struct GoogleOneTapNonceResponse {
    /// Single-use nonce bound to the caller's anonymous id (TTL-limited).
    pub nonce: String,
}

impl IntoResponse for GoogleOneTapNonceResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
