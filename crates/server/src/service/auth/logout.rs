use crate::errors::errors::{Errors, ServiceResult};
use crate::service::auth::session::SessionService;
use redis::aio::ConnectionManager;

pub async fn service_logout(redis: &ConnectionManager, session_id: &str) -> ServiceResult<()> {
    // 먼저 세션이 유효한지 확인
    let _session = SessionService::get_session(redis, session_id)
        .await?
        .ok_or(Errors::UserUnauthorized)?;

    // 세션 삭제
    SessionService::delete_session(redis, session_id).await?;

    Ok(())
}
