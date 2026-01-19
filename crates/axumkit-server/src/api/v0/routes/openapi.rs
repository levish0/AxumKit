use super::action_logs::openapi::ActionLogsOpenApi;
use super::auth::openapi::AuthApiDoc;
use super::posts::openapi::PostsApiDoc;
use super::search::openapi::SearchApiDoc;
use super::stream::openapi::StreamOpenApi;
use super::user::openapi::UserApiDoc;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi()]
pub struct V0ApiDoc;

impl V0ApiDoc {
    pub fn merged() -> utoipa::openapi::OpenApi {
        let mut openapi = Self::openapi();
        openapi.merge(AuthApiDoc::openapi());
        openapi.merge(UserApiDoc::openapi());
        openapi.merge(PostsApiDoc::openapi());
        openapi.merge(SearchApiDoc::openapi());
        openapi.merge(ActionLogsOpenApi::openapi());
        openapi.merge(StreamOpenApi::openapi());
        openapi
    }
}
