use axumkit_entity::users::{Entity as UserEntity, Model as UserModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, QuerySelect};
use uuid::Uuid;

pub async fn repository_get_user_by_id<C>(conn: &C, id: Uuid) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find_by_id(id).one(conn).await?;

    user.ok_or(Errors::UserNotFound)
}

/// Get user by id with row-level lock (SELECT ... FOR UPDATE).
/// Used to serialize critical per-user mutations.
pub async fn repository_get_user_by_id_for_update<C>(
    conn: &C,
    id: Uuid,
) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find_by_id(id)
        .lock_exclusive()
        .one(conn)
        .await?;

    user.ok_or(Errors::UserNotFound)
}
