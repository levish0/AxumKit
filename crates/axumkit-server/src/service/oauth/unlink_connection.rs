use crate::repository::oauth::count_oauth_connections::repository_count_oauth_connections;
use crate::repository::oauth::delete_oauth_connection::repository_delete_oauth_connection;
use crate::repository::user::get_by_id::repository_get_user_by_id;
use sea_orm::ConnectionTrait;
use uuid::Uuid;
use axumkit_entity::common::OAuthProvider;
use axumkit_errors::errors::{Errors, ServiceResult};

/// OAuth 연결을 해제합니다.
///
/// 마지막 인증 수단 보호:
/// - OAuth 연결이 1개뿐이고 비밀번호가 설정되지 않은 경우 해제 불가
pub async fn service_unlink_oauth<C>(
    conn: &C,
    user_id: Uuid,
    provider: OAuthProvider,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    // 1. 사용자 정보 확인
    let user = repository_get_user_by_id(conn, user_id).await?;

    // 2. OAuth 연결 개수 확인
    let oauth_count = repository_count_oauth_connections(conn, user_id).await?;

    // 3. 마지막 인증 수단 보호: OAuth 1개 + 비밀번호 없음 = 해제 불가
    if oauth_count == 1 && user.password.is_none() {
        return Err(Errors::OauthCannotUnlinkLastConnection);
    }

    // 4. OAuth 연결 삭제
    repository_delete_oauth_connection(conn, user_id, provider).await?;

    Ok(())
}
