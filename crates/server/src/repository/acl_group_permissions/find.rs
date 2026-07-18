use entity::acl_group_permissions::{Column, Entity, Model};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

/// Lists a group's permission grants (stable order for admin listings).
pub async fn repository_find_permissions_for_group<C>(
    conn: &C,
    group_id: Uuid,
) -> Result<Vec<Model>, Errors>
where
    C: ConnectionTrait,
{
    let rows = Entity::find()
        .filter(Column::GroupId.eq(group_id))
        .order_by_asc(Column::Permission)
        .all(conn)
        .await?;

    Ok(rows)
}

/// Lists permission grants for a set of groups (the per-request permission
/// load for a user's memberships).
pub async fn repository_find_permissions_for_groups<C>(
    conn: &C,
    group_ids: &[Uuid],
) -> Result<Vec<Model>, Errors>
where
    C: ConnectionTrait,
{
    if group_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = Entity::find()
        .filter(Column::GroupId.is_in(group_ids.iter().copied()))
        .all(conn)
        .await?;

    Ok(rows)
}
