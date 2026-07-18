use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use constants::NotificationAction;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for notification action preference response.
pub struct NotificationActionPreferenceResponse {
    pub action: NotificationAction,
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
/// Response payload for notification action preference list response.
pub struct NotificationActionPreferenceListResponse {
    pub preferences: Vec<NotificationActionPreferenceResponse>,
}

impl IntoResponse for NotificationActionPreferenceListResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, Json(self)).into_response()
    }
}
