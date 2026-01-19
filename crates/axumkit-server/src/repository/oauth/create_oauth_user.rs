use axumkit_entity::users::{ActiveModel as UserActiveModel, Model as UserModel};
use axumkit_errors::errors::Errors;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// OAuth를 통한 새 유저 생성 (비밀번호 없음, 이메일 인증 완료 상태)
pub async fn repository_create_oauth_user<C>(
    conn: &C,
    email: &str,
    display_name: &str,
    handle: &str,
    profile_image: Option<String>,
) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let new_user = UserActiveModel {
        id: Default::default(),
        display_name: Set(display_name.to_string()),
        handle: Set(handle.to_string()),
        bio: Set(None),
        email: Set(email.to_string()),
        password: Set(None),                // OAuth 유저는 비밀번호 없음
        verified_at: Set(Some(Utc::now())), // OAuth 제공자가 이미 이메일 검증함
        profile_image: Set(profile_image),
        banner_image: Set(None),
        totp_secret: Set(None),
        totp_enabled_at: Set(None),
        totp_backup_codes: Set(None),
        created_at: Default::default(),
    };

    let user = new_user.insert(conn).await?;

    Ok(user)
}
