use crate::dto::auth::LoginRequest;
use crate::errors::errors::{Errors, ServiceResult};
use crate::repository::user::repository_find_user_by_email;
use crate::service::auth::session::SessionService;
use crate::utils::crypto::password::verify_password;
use redis::aio::ConnectionManager;
use sea_orm::DatabaseConnection;

pub async fn service_login(
    conn: &DatabaseConnection,
    redis: &ConnectionManager,
    payload: LoginRequest,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<String> {
    // 사용자 검증
    let user = repository_find_user_by_email(conn, payload.email.clone())
        .await?
        .ok_or(Errors::UserNotFound)?;

    // 비밀번호 검증
    let password_hash = user.password.ok_or(Errors::UserPasswordNotSet)?;
    verify_password(&payload.password, &password_hash)?;

    // 세션 생성
    let session =
        SessionService::create_session(redis, user.id.to_string(), user_agent, ip_address).await?;

    Ok(session.session_id)
}
