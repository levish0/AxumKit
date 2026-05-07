use crate::repository::moderation::{
    ModerationLogFilter, repository_exists_newer_moderation_log,
    repository_exists_older_moderation_log, repository_find_moderation_logs,
};
use dto::moderation::{
    ListModerationLogsRequest, ListModerationLogsResponse, ModerationLogListItem,
};
use dto::pagination::CursorDirection;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

pub async fn service_list_moderation_logs(
    conn: &DatabaseConnection,
    payload: ListModerationLogsRequest,
) -> ServiceResult<ListModerationLogsResponse> {
    let limit = payload.limit;
    let is_newer = payload.cursor_direction == Some(CursorDirection::Newer);

    let filter = ModerationLogFilter {
        actor_id: payload.actor_id,
        resource_type: payload.resource_type,
        resource_id: payload.resource_id,
        actions: payload.actions,
    };

    let mut logs = repository_find_moderation_logs(
        conn,
        &filter,
        payload.cursor_id,
        payload.cursor_direction,
        limit,
    )
    .await?;

    let (has_newer, has_older) = if logs.is_empty() {
        (false, false)
    } else {
        let first_id = logs.first().unwrap().id;
        let last_id = logs.last().unwrap().id;
        if is_newer {
            let has_newer = repository_exists_newer_moderation_log(conn, &filter, last_id).await?;
            let has_older = repository_exists_older_moderation_log(conn, &filter, first_id).await?;
            (has_newer, has_older)
        } else {
            let has_newer = repository_exists_newer_moderation_log(conn, &filter, first_id).await?;
            let has_older = repository_exists_older_moderation_log(conn, &filter, last_id).await?;
            (has_newer, has_older)
        }
    };

    if is_newer {
        logs.reverse();
    }

    let data: Vec<ModerationLogListItem> =
        logs.into_iter().map(ModerationLogListItem::from).collect();

    Ok(ListModerationLogsResponse {
        data,
        has_newer,
        has_older,
    })
}
