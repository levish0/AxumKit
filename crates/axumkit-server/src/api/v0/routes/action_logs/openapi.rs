use axumkit_dto::action_logs::{ActionLogListResponse, ActionLogResponse, GetActionLogsRequest};
use utoipa::OpenApi;

use super::recent_changes::__path_get_action_logs;

#[derive(OpenApi)]
#[openapi(
    paths(get_action_logs),
    components(schemas(
        GetActionLogsRequest,
        ActionLogResponse,
        ActionLogListResponse,
    )),
    tags(
        (name = "Action Logs", description = "User activity logs")
    )
)]
pub struct ActionLogsOpenApi;
