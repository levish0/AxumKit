use crate::service::auth::session::SessionService;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use tracing::info;

/// Logs out the current session.
///
/// # Responsibilities
/// Deletes the server-side session matching the session ID.
///
/// # Related
/// - `SessionService::delete_session`
///
/// # Errors
/// - Returns Redis/storage errors when session deletion fails.
pub async fn service_logout(redis: &ConnectionManager, session_id: &str) -> ServiceResult<()> {
    // Delete the session (delete_session validates it internally)
    SessionService::delete_session(redis, session_id).await?;

    info!(session_id = %session_id, "Logout");

    Ok(())
}
