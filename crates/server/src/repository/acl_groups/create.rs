use entity::acl_groups::{ActiveModel as AclGroupActiveModel, Model as AclGroupModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// Creates a (non-system) ACL group.
pub async fn repository_create_acl_group<C>(
    conn: &C,
    name: String,
    description: Option<String>,
) -> Result<AclGroupModel, Errors>
where
    C: ConnectionTrait,
{
    let group = AclGroupActiveModel {
        name: Set(name),
        description: Set(description),
        is_system: Set(false),
        ..Default::default()
    };

    Ok(group.insert(conn).await?)
}
