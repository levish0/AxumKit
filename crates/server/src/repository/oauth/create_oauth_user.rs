use chrono::Utc;
use entity::users::{ActiveModel as UserActiveModel, Model as UserModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// Create a new user via OAuth (no password, email verified)
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
        password: Set(None),                // OAuth users have no password
        verified_at: Set(Some(Utc::now())), // OAuth provider already verified email
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
