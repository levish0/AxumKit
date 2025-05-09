use crate::service::error::errors::ErrorResponse;
use crate::dto::user_dto::UserInfoResponse;
use crate::dto::user_dto::CreateUserRequest;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::v0::routes::user::user::get_user,
        crate::api::v0::routes::user::user::create_user,
    ),
    components(schemas(
        CreateUserRequest,
        UserInfoResponse,
        ErrorResponse
    )),
    tags(
        (name = "User")
    )
)]
pub struct ApiDoc;