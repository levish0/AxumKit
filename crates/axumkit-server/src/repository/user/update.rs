use axumkit_entity::users::{
    ActiveModel as UserActiveModel, Entity as UserEntity, Model as UserModel,
};
use axumkit_errors::errors::Errors;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, IntoActiveModel, Set};
use uuid::Uuid;

/// 사용자 업데이트 파라미터
/// - `Option<T>`: None = 변경 안 함, Some(value) = 값으로 변경
/// - `Option<Option<T>>`: None = 변경 안 함, Some(None) = NULL로 설정, Some(Some(value)) = 값으로 설정
///
/// # 사용 예시
/// ```ignore
/// repository_update_user(conn, user_id, UserUpdateParams {
///     totp_secret: Some(Some(secret)),
///     totp_enabled_at: Some(Some(Utc::now())),
///     ..Default::default()
/// }).await?;
/// ```
#[derive(Default)]
pub struct UserUpdateParams {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<Option<String>>,
    pub password: Option<Option<String>>,
    pub verified_at: Option<Option<DateTimeUtc>>,
    pub profile_image: Option<Option<String>>,
    pub banner_image: Option<Option<String>>,
    pub totp_secret: Option<Option<String>>,
    pub totp_enabled_at: Option<Option<DateTimeUtc>>,
    pub totp_backup_codes: Option<Option<Vec<String>>>,
}

/// 범용 사용자 정보 업데이트
pub async fn repository_update_user<C>(
    conn: &C,
    user_id: Uuid,
    params: UserUpdateParams,
) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find_by_id(user_id)
        .one(conn)
        .await?
        .ok_or(Errors::UserNotFound)?;

    let mut user_active: UserActiveModel = user.into_active_model();

    if let Some(email) = params.email {
        user_active.email = Set(email);
    }
    if let Some(display_name) = params.display_name {
        user_active.display_name = Set(display_name);
    }
    if let Some(bio) = params.bio {
        user_active.bio = Set(bio);
    }
    if let Some(password) = params.password {
        user_active.password = Set(password);
    }
    if let Some(verified_at) = params.verified_at {
        user_active.verified_at = Set(verified_at);
    }
    if let Some(profile_image) = params.profile_image {
        user_active.profile_image = Set(profile_image);
    }
    if let Some(banner_image) = params.banner_image {
        user_active.banner_image = Set(banner_image);
    }
    if let Some(totp_secret) = params.totp_secret {
        user_active.totp_secret = Set(totp_secret);
    }
    if let Some(totp_enabled_at) = params.totp_enabled_at {
        user_active.totp_enabled_at = Set(totp_enabled_at);
    }
    if let Some(totp_backup_codes) = params.totp_backup_codes {
        user_active.totp_backup_codes = Set(totp_backup_codes);
    }

    let updated_user = user_active.update(conn).await?;
    Ok(updated_user)
}
