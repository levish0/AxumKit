use crate::service::action_logs::service_get_action_logs;
use crate::state::AppState;
use axum::extract::State;
use dto::action_logs::{ActionLogListResponse, GetActionLogsRequest};
use dto::validator::query_validator::ValidatedQuery;
use errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/action-logs",
    params(GetActionLogsRequest),
    responses(
        (status = 200, description = "Action logs retrieved successfully", body = ActionLogListResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error"),
        (status = 500, description = "Internal Server Error - Database error")
    ),
    tag = "Action Logs"
)]
pub async fn get_action_logs(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<GetActionLogsRequest>,
) -> Result<ActionLogListResponse, Errors> {
    service_get_action_logs(&state.db, payload).await
}
