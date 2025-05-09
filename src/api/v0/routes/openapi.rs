use crate::dto::user_dto::CreateUserRequest;
use crate::dto::user_dto::UserInfoResponse;
use crate::service::error::errors::ErrorResponse;
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
