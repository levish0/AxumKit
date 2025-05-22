use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Deserialize, Validate, Debug, ToSchema)]
pub struct CreateUserRequest {
    #[validate(length(
        min = 3,
        max = 20,
        message = "Name must be between 3 and 20 characters."
    ))]
    pub name: String,
    #[validate(length(
        min = 3,
        max = 20,
        message = "Handle must be between 3 and 20 characters."
    ))]
    pub handle: String,
    #[validate(length(
        min = 6,
        max = 20,
        message = "Password must be between 6 and 20 characters."
    ))]
    pub password: String,
    #[validate(email)]
    pub email: String,
}

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
