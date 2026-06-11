use crate::repository::action_logs::{
    ActionLogFilter, repository_exists_newer_action_log, repository_exists_older_action_log,
    repository_find_action_logs,
};
use dto::action_logs::{ActionLogListResponse, ActionLogResponse, GetActionLogsRequest};
use dto::pagination::CursorDirection;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// 액션 로그 목록을 커서 기반으로 조회한다.
///
/// # 역할
/// - 요청 필터를 `ActionLogFilter`로 변환한다.
/// - 커서 페이지를 조회하고 `has_newer`/`has_older`를 계산한다.
/// - 결과를 API 응답 DTO로 매핑한다.
///
/// # 연계
/// - `repository_find_action_logs`
/// - `repository_exists_newer_action_log`
/// - `repository_exists_older_action_log`
///
/// # Errors
/// - 조회 실패 시 DB/저장소 에러를 반환한다.
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
