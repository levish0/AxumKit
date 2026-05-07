use entity::users::{ActiveModel as UserActiveModel, Model as UserModel};
use errors::errors::Errors;

use crate::utils::crypto::password::hash_password;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub async fn repository_create_user<C>(
    conn: &C,
    email: String,
    handle: String,
    display_name: String,
    password: String,
) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let hashed_password = hash_password(&password)?;

    let new_user = UserActiveModel {
        id: Default::default(),
        display_name: Set(display_name),
        handle: Set(handle),
        bio: Set(None),
        email: Set(email),
        password: Set(Some(hashed_password)),
        verified_at: Set(None),
        profile_image: Set(None),
        banner_image: Set(None),
        totp_secret: Set(None),
        totp_enabled_at: Set(None),
        totp_backup_codes: Set(None),
        created_at: Default::default(),
    };

    let user = new_user.insert(conn).await?;

    Ok(user)
}

/// Create a user with a pre-hashed password (used by email verification flow).
pub async fn repository_create_user_with_password_hash<C>(
    conn: &C,
    email: String,
    handle: String,
    display_name: String,
    password_hash: String,
) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let new_user = UserActiveModel {
        id: Default::default(),
        display_name: Set(display_name),
        handle: Set(handle),
        bio: Set(None),
        email: Set(email),
        password: Set(Some(password_hash)),
        verified_at: Set(None),
        profile_image: Set(None),
        banner_image: Set(None),
        totp_secret: Set(None),
        totp_enabled_at: Set(None),
        totp_backup_codes: Set(None),
        created_at: Default::default(),
    };

    let user = new_user.insert(conn).await?;
    Ok(user)
}
