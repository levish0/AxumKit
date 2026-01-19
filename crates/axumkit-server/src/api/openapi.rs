use super::health::openapi::HealthApiDoc;
use super::v0::routes::openapi::V0ApiDoc;
use axumkit_errors::errors::ErrorResponse;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

#[derive(OpenApi)]
#[openapi(
    components(
        schemas(
            ErrorResponse,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

impl ApiDoc {
    pub fn merged() -> utoipa::openapi::OpenApi {
        let mut openapi = Self::openapi();
        openapi.merge(HealthApiDoc::openapi());
        openapi.merge(V0ApiDoc::merged());
        openapi
    }
}

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "anonymous_id_cookie",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("anonymous_user_id"))),
            );
            components.add_security_scheme(
                "session_id_cookie",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("session_id"))),
            );
        }
    }
}
