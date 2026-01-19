use axumkit_entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn repository_find_user_by_email<C>(
    conn: &C,
    email: String,
) -> Result<Option<UserModel>, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(UsersColumn::Email.eq(email))
        .one(conn)
        .await?;

    Ok(user)
}
