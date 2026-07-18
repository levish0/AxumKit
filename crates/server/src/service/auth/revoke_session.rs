use crate::service::auth::session::SessionService;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use tracing::info;
use uuid::Uuid;

/// Forcibly terminates a single session owned by the user.
///
/// # Role
/// Verifies session ownership, then deletes it from Redis.
/// Missing sessions and sessions owned by another user both map to `Errors::NotFound`.
///
/// # Related
/// - `SessionService::revoke_user_session`
///
/// # Errors
/// - `Errors::NotFound` if the target session is missing or owned by another user
/// - `Errors::SysInternalError` on Redis failure
pub async fn service_revoke_session(
    redis: &ConnectionManager,
    user_id: Uuid,
    management_id: Uuid,
) -> ServiceResult<()> {
    SessionService::revoke_user_session(redis, &user_id.to_string(), &management_id.to_string())
        .await?;

    info!(user_id = %user_id, management_id = %management_id, "Session revoked");

    Ok(())
}
