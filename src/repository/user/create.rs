use crate::entity::users::{ActiveModel as UserActiveModel, Model as UserModel};
use crate::errors::errors::Errors;
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
        is_verified: Set(false),
        is_banned: Set(false),
        profile_image: Set(None),
        banner_image: Set(None),
        created_at: Default::default(),
    };

    let user = new_user.insert(conn).await?;

    Ok(user)
}
