use entity::groups::Entity as GroupEntity;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

/// Deletes an ACL group by id. Returns whether a row was deleted.
///
/// Members cascade; rules referencing the group as a condition RESTRICT the
/// delete at the DB level — callers must detach rules first.
pub async fn repository_delete_group<C>(conn: &C, group_id: Uuid) -> Result<bool, Errors>
where
    C: ConnectionTrait,
{
    let result = GroupEntity::delete_by_id(group_id).exec(conn).await?;
    Ok(result.rows_affected > 0)
}
