use super::auth::openapi::AuthApiDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi()]
pub struct V0ApiDoc;

impl V0ApiDoc {
    pub fn merged() -> utoipa::openapi::OpenApi {
        let mut openapi = Self::openapi();
        openapi.merge(AuthApiDoc::openapi());
        openapi
    }
}