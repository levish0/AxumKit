use entity::users::{ActiveModel as UserActiveModel, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::prelude::DateTimeUtc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, IntoActiveModel, Set};
use uuid::Uuid;

use crate::utils::email::normalize_email;

/// User update parameters
/// - `Option<T>`: None = leave unchanged, Some(value) = set to value
/// - `Option<Option<T>>`: None = leave unchanged, Some(None) = set to NULL, Some(Some(value)) = set to value
///
/// # Example
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
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<Option<String>>,
    pub password: Option<Option<String>>,
    pub profile_image: Option<Option<String>>,
    pub banner_image: Option<Option<String>>,
    pub totp_secret: Option<Option<String>>,
    pub totp_enabled_at: Option<Option<DateTimeUtc>>,
    pub totp_backup_codes: Option<Option<Vec<String>>>,
    pub deleted_at: Option<Option<DateTimeUtc>>,
}

/// General-purpose user info update
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
        user_active.email = Set(normalize_email(&email));
    }
    if let Some(handle) = params.handle {
        user_active.handle = Set(handle);
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
    if let Some(deleted_at) = params.deleted_at {
        user_active.deleted_at = Set(deleted_at);
    }

    let updated_user = user_active.update(conn).await?;
    Ok(updated_user)
}
