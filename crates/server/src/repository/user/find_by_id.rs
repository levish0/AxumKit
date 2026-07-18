use entity::users::{Entity as UserEntity, Model as UserModel};
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

/// Looks up whether a user exists by user ID.
///
/// # Role
/// Performs a single-row query by ID and returns `Option<UserModel>`.
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn repository_find_user_by_id<C>(conn: &C, id: Uuid) -> Result<Option<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find_by_id(id).one(conn).await?;

    Ok(user)
}
