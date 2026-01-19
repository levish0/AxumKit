use super::actions::__path_stream_actions;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(stream_actions))]
pub struct StreamOpenApi;
