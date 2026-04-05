use crate::service::moderation::service_list_moderation_logs;
use crate::state::AppState;
use axum::extract::State;
use axumkit_dto::moderation::{ListModerationLogsRequest, ListModerationLogsResponse};
use axumkit_dto::validator::query_validator::ValidatedQuery;
use axumkit_errors::errors::Errors;

#[utoipa::path(
    get,
    path = "/v0/moderation/logs",
    params(ListModerationLogsRequest),
    responses(
        (status = 200, description = "Moderation logs retrieved successfully", body = ListModerationLogsResponse),
        (status = 400, description = "Bad request - Invalid query parameters or validation error"),
        (status = 500, description = "Internal Server Error - Database error")
    ),
    tag = "Moderation"
)]
pub async fn list_moderation_logs(
    State(state): State<AppState>,
    ValidatedQuery(payload): ValidatedQuery<ListModerationLogsRequest>,
) -> Result<ListModerationLogsResponse, Errors> {
    service_list_moderation_logs(&state.db, payload).await
}
