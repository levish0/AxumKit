use entity::acl_groups::{
    Column as AclGroupColumn, Entity as AclGroupEntity, Model as AclGroupModel,
};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use uuid::Uuid;

/// Finds one ACL group by id.
pub async fn repository_find_acl_group_by_id<C>(
    conn: &C,
    group_id: Uuid,
) -> Result<Option<AclGroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupEntity::find_by_id(group_id).one(conn).await?)
}

/// Finds one ACL group by its unique name.
pub async fn repository_find_acl_group_by_name<C>(
    conn: &C,
    name: &str,
) -> Result<Option<AclGroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupEntity::find()
        .filter(AclGroupColumn::Name.eq(name))
        .one(conn)
        .await?)
}

/// Lists all ACL groups (name order).
pub async fn repository_list_acl_groups<C>(conn: &C) -> Result<Vec<AclGroupModel>, Errors>
where
    C: ConnectionTrait,
{
    Ok(AclGroupEntity::find()
        .order_by_asc(AclGroupColumn::Name)
        .all(conn)
        .await?)
}
