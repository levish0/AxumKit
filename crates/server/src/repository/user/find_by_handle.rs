use entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

/// Looks up whether a user exists by handle.
///
/// # Role
/// Performs a single-row query filtered by `handle` and returns `Option<UserModel>`.
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_find_user_by_handle<C>(
    conn: &C,
    handle: String,
) -> Result<Option<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(UsersColumn::Handle.eq(handle))
        .one(conn)
        .await?;

    Ok(user)
}
