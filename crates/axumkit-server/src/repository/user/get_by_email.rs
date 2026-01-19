use axumkit_entity::users::{Column as UsersColumn, Entity as UserEntity, Model as UserModel};
use axumkit_errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn repository_get_user_by_email<C>(conn: &C, email: String) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find()
        .filter(UsersColumn::Email.eq(email))
        .one(conn)
        .await?;

    user.ok_or(Errors::UserNotFound)
}
