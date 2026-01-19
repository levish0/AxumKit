use crate::service::auth::session::SessionService;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;

pub async fn service_logout(redis: &ConnectionManager, session_id: &str) -> ServiceResult<()> {
    // 세션 삭제 (delete_session 내부에서 유효성 확인)
    SessionService::delete_session(redis, session_id).await?;

    Ok(())
}
