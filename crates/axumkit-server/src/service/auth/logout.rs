use crate::service::auth::session::SessionService;
use redis::aio::ConnectionManager;
use tracing::info;
use axumkit_errors::errors::ServiceResult;

pub async fn service_logout(redis: &ConnectionManager, session_id: &str) -> ServiceResult<()> {
    SessionService::delete_session(redis, session_id).await?;

    info!(session_id = %session_id, "Logout");

    Ok(())
}

