use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct UserInfoResponse {
    pub name: String,
    pub handle: String,
    pub email: String,
}

impl IntoResponse for UserInfoResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
