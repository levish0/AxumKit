use entity::users::Entity as UsersEntity;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

/// Hard-delete a user row by id.
///
/// FK cascades remove the user's dependent rows (OAuth connections, roles, bans, known devices);
/// `action_logs.actor_id` is set null and `auth_events` (no FK) are retained for forensics.
pub async fn repository_delete_user<C: ConnectionTrait>(
    conn: &C,
    user_id: Uuid,
) -> Result<(), Errors> {
    UsersEntity::delete_by_id(user_id).exec(conn).await?;
    Ok(())
}
