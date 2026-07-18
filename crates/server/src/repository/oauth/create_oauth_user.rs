use entity::users::{ActiveModel as UserActiveModel, Model as UserModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, Set, SqlErr};

use crate::utils::email::normalize_email;

/// Map an insert failure to a meaningful 409 when it is a unique-constraint
/// violation (a concurrent signup won the race after the in-transaction
/// re-checks), instead of leaking it as a generic 500. The constraint detail
/// names the index (`users_email_lower_key` / `users_handle_key`).
fn map_create_user_db_err(err: DbErr) -> Errors {
    match err.sql_err() {
        Some(SqlErr::UniqueConstraintViolation(detail)) => {
            let detail = detail.to_lowercase();
            if detail.contains("email") {
                Errors::OauthEmailAlreadyExists
            } else if detail.contains("handle") {
                Errors::UserHandleAlreadyExists
            } else {
                Errors::DatabaseError(err.to_string())
            }
        }
        _ => Errors::DatabaseError(err.to_string()),
    }
}

/// Creates a new user via OAuth (no password).
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
        email: Set(normalize_email(email)),
        password: Set(None),
        profile_image: Set(profile_image),
        banner_image: Set(None),
        totp_secret: Set(None),
        totp_enabled_at: Set(None),
        totp_backup_codes: Set(None),
        created_at: Default::default(),
        deleted_at: Set(None),
    };

    let user = new_user
        .insert(conn)
        .await
        .map_err(map_create_user_db_err)?;

    Ok(user)
}
