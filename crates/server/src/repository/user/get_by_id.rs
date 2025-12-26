use crate::entity::users::{Entity as UserEntity, Model as UserModel};
use crate::errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

pub async fn repository_get_user_by_id<C>(conn: &C, id: Uuid) -> Result<UserModel, Errors>
where
    C: ConnectionTrait,
{
    let user = UserEntity::find_by_id(id).one(conn).await?;

    user.ok_or(Errors::UserNotFound)
}
