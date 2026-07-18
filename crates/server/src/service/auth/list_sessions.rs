use crate::service::auth::session::SessionService;
use dto::auth::response::SessionInfo;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use uuid::Uuid;

/// Fetches the user's active sessions and converts them into response DTOs.
///
/// # Responsibilities
/// - Maps the Redis `Session` payload into the response `SessionInfo`.
/// - Marks the entry matching the caller's session ID with `is_current = true`.
/// - The response includes only the public management ID, never the bearer session ID.
///
/// # Related
/// - `SessionService::list_user_sessions`
///
/// # Errors
/// - Redis lookup failures are propagated to the caller.
pub async fn service_list_sessions(
    redis: &ConnectionManager,
    user_id: Uuid,
    current_session_id: &str,
) -> ServiceResult<Vec<SessionInfo>> {
    let sessions = SessionService::list_user_sessions(redis, &user_id.to_string()).await?;

    let infos = sessions
        .into_iter()
        .map(|s| {
            // The stored management_id is a UUIDv7 string, but if corrupted data slips in,
            // fall back to the nil UUID on parse failure so the whole list response doesn't fail.
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
