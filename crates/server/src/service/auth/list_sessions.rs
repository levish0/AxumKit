use crate::service::auth::session::SessionService;
use dto::auth::response::SessionInfo;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use uuid::Uuid;

pub async fn service_list_sessions(
    redis: &ConnectionManager,
    user_id: Uuid,
    current_session_id: &str,
) -> ServiceResult<Vec<SessionInfo>> {
    let sessions = SessionService::list_user_sessions(redis, &user_id.to_string()).await?;

    let infos = sessions
        .into_iter()
        .map(|s| {
            let is_current = s.session_id == current_session_id;
            let management_id = Uuid::parse_str(&s.management_id).unwrap_or(Uuid::nil());
            SessionInfo {
                management_id,
                created_at: s.created_at,
                expires_at: s.expires_at,
                max_expires_at: s.max_expires_at,
                user_agent: s.user_agent,
                ip_address: s.ip_address,
                is_current,
            }
        })
        .collect();

    Ok(infos)
}
