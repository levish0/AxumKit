use crate::entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use crate::errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn repository_get_user_by_handle<C>(conn: &C, handle: String) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(UsersColumn::Handle.eq(handle))
        .one(conn)
        .await?;

    user.ok_or(Errors::UserNotFound)
}
