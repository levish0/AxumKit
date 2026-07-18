use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for notification preference response.
pub struct NotificationPreferenceResponse {
    pub email_enabled: bool,
    pub push_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

impl IntoResponse for NotificationPreferenceResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
