use axumkit_entity::users::{Column as UserColumn, Entity as UserEntity, Model as UserModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn repository_find_users_by_ids<C>(
    conn: &C,
    ids: &[Uuid],
) -> Result<Vec<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let users = UserEntity::find()
        .filter(UserColumn::Id.is_in(ids.to_vec()))
        .all(conn)
        .await?;

    Ok(users)
}
