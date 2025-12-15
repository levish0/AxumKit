use crate::entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use crate::errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

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
