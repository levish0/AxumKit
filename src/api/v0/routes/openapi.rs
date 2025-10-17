use crate::dto::auth::request::login::AuthLoginRequest;
use crate::dto::auth::response::jwt::AuthJWTResponse;
use crate::dto::user::request::create::CreateUserRequest;
use crate::dto::user::response::info::UserInfoResponse;
use crate::errors::errors::ErrorResponse;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::v0::routes::auth::auth::login,
        crate::api::v0::routes::auth::auth::refresh,
        crate::api::v0::routes::user::user::get_user,
        crate::api::v0::routes::user::user::create_user,
        crate::api::v0::routes::user::user::get_profile,
    ),
    components(
        schemas(
            AuthLoginRequest,
            AuthJWTResponse,
            CreateUserRequest,
            UserInfoResponse,
            ErrorResponse
        )
    ),
    tags(
        (name = "User", description = "User management endpoints"),
        (name = "Auth", description = "Authentication endpoints")
    ),
    modifiers(&SecurityAddon) // 보안 스키마 등록
)]
pub struct ApiDoc;

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}
