use crate::dto::oauth::internal::oauth_user_result::OAuthUserResult;
use crate::entity::common::OAuthProvider;
use crate::errors::errors::Errors;
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::create_oauth_user::repository_create_oauth_user;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use sea_orm::ConnectionTrait;

/// OAuth 제공자로부터 받은 정보로 사용자를 찾거나 생성합니다.
///
/// # Arguments
/// * `conn` - 데이터베이스 연결 (트랜잭션 내부에서 호출되어야 함)
/// * `provider` - OAuth 제공자 (Google, GitHub 등)
/// * `provider_user_id` - OAuth 제공자에서의 사용자 ID
/// * `email` - 사용자 이메일
/// * `display_name` - 사용자 표시 이름
/// * `handle` - 사용자 핸들 (신규 사용자 생성 시 필수)
/// * `profile_image` - 프로필 이미지 URL (선택사항)
///
/// # Returns
/// * `OAuthUserResult` - 사용자 모델과 신규 사용자 여부
pub async fn service_find_or_create_oauth_user<C>(
    conn: &C,
    provider: OAuthProvider,
    provider_user_id: &str,
    email: &str,
    display_name: &str,
    handle: Option<&str>,
    profile_image: Option<String>,
) -> Result<OAuthUserResult, Errors>
where
    C: ConnectionTrait,
{
    // 1. 기존 OAuth 연결이 있는지 확인
    if let Some(user) =
        repository_find_user_by_oauth(conn, provider.clone(), provider_user_id).await?
    {
        return Ok(OAuthUserResult {
            user,
            is_new_user: false,
        });
    }

    // 2. 신규 사용자 - handle 필수
    let handle = handle.ok_or(Errors::OauthHandleRequired)?;

    // 3. handle 중복 확인
    if repository_find_user_by_handle(conn, handle.to_string())
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // 4. 새 사용자 생성
    let new_user =
        repository_create_oauth_user(conn, email, display_name, handle, profile_image).await?;

    // 5. OAuth 연결 생성
    repository_create_oauth_connection(conn, &new_user.id, provider, provider_user_id).await?;

    Ok(OAuthUserResult {
        user: new_user,
        is_new_user: true,
    })
}
