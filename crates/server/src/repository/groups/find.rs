use entity::groups::{Column as GroupColumn, Entity as GroupEntity, Model as GroupModel};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

/// Finds one ACL group by id.
pub async fn repository_find_group_by_id<C>(
    conn: &C,
    group_id: Uuid,
) -> Result<Option<GroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupEntity::find_by_id(group_id).one(conn).await?)
}

/// Finds one ACL group by its unique name.
pub async fn repository_find_group_by_name<C>(
    conn: &C,
    name: &str,
) -> Result<Option<GroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupEntity::find()
        .filter(GroupColumn::Name.eq(name))
        .one(conn)
        .await?)
}

/// Lists all ACL groups (name order).
pub async fn repository_list_groups<C>(conn: &C) -> Result<Vec<GroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(GroupEntity::find()
        .order_by_asc(GroupColumn::Name)
        .all(conn)
        .await?)
}
