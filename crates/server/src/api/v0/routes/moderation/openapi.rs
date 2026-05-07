use dto::moderation::{
    ListModerationLogsRequest, ListModerationLogsResponse, ModerationLogListItem,
};
use utoipa::OpenApi;

use super::list_logs::__path_list_moderation_logs;

#[derive(OpenApi)]
#[openapi(
    paths(list_moderation_logs),
    components(schemas(
        ListModerationLogsRequest,
        ModerationLogListItem,
        ListModerationLogsResponse,
    )),
    tags(
        (name = "Moderation", description = "Moderation logs")
    )
)]
pub struct ModerationOpenApi;
