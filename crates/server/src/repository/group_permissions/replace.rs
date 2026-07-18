use entity::group_permissions::{ActiveModel, Column, Entity};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

/// Replaces a group's permission grants with the given list (whole-list
/// replacement: list state is the API contract, so admins submit the desired
/// end state instead of diffing).
///
/// Caller owns the transaction and has already validated the codenames.
pub async fn repository_replace_group_permissions<C>(
    conn: &C,
    group_id: Uuid,
    permissions: &[String],
    created_by: Option<Uuid>,
) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    Entity::delete_many()
        .filter(Column::GroupId.eq(group_id))
        .exec(conn)
        .await?;

    if permissions.is_empty() {
        return Ok(());
    }

    let rows = permissions.iter().map(|permission| ActiveModel {
        id: Default::default(),
        group_id: Set(group_id),
        permission: Set(permission.clone()),
        created_by: Set(created_by),
        created_at: Default::default(),
    });

    Entity::insert_many(rows).exec(conn).await?;

    Ok(())
}
