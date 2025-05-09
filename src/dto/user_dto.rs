use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(
        min = 3,
        max = 20,
        message = "Username must be between 3 and 20 characters"
    ))]
    pub username: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters"
    ))]
    pub password: String,
    #[validate(email)]
    pub email: String,
}

#[derive(Deserialize, Serialize)]
pub struct UserInfoResponse {
    pub username: String,
    pub email: String,
}

impl IntoResponse for UserInfoResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
