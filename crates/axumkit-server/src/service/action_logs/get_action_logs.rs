use crate::repository::action_logs::{
    ActionLogFilter, repository_exists_newer_action_log, repository_exists_older_action_log,
    repository_find_action_logs,
};
use axumkit_dto::action_logs::{ActionLogListResponse, ActionLogResponse, GetActionLogsRequest};
use axumkit_dto::pagination::CursorDirection;
use axumkit_errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

pub async fn service_get_action_logs(
    conn: &DatabaseConnection,
    payload: GetActionLogsRequest,
) -> ServiceResult<ActionLogListResponse> {
    let limit = payload.limit;
    let is_newer = payload.cursor_direction == Some(CursorDirection::Newer);

    let filter = ActionLogFilter {
        actor_id: payload.user_id,
        resource_id: payload.resource_id,
        resource_type: payload.resource_type,
        actions: payload.actions,
        ..Default::default()
    };

    let mut logs = repository_find_action_logs(
        conn,
        &filter,
        payload.cursor_id,
        payload.cursor_direction,
        limit,
    )
    .await?;

    // Calculate has_newer / has_older
    // Note: When direction=Newer, repository returns ASC order (first=oldest, last=newest)
    //       When direction=Older, repository returns DESC order (first=newest, last=oldest)
    let (has_newer, has_older) = if logs.is_empty() {
        (false, false)
    } else {
        let first_id = logs.first().unwrap().id;
        let last_id = logs.last().unwrap().id;
        if is_newer {
            let has_newer = repository_exists_newer_action_log(conn, &filter, last_id).await?;
            let has_older = repository_exists_older_action_log(conn, &filter, first_id).await?;
            (has_newer, has_older)
        } else {
            let has_newer = repository_exists_newer_action_log(conn, &filter, first_id).await?;
            let has_older = repository_exists_older_action_log(conn, &filter, last_id).await?;
            (has_newer, has_older)
        }
    };

    // Reverse if Newer direction
    if is_newer {
        logs.reverse();
    }

    let data: Vec<ActionLogResponse> = logs.into_iter().map(ActionLogResponse::from).collect();

    Ok(ActionLogListResponse {
        data,
        has_newer,
        has_older,
    })
}
