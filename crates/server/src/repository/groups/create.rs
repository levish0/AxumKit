use entity::groups::{ActiveModel as GroupActiveModel, Model as GroupModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

/// Creates a (non-system) ACL group.
pub async fn repository_create_group<C>(
    conn: &C,
    name: String,
    description: Option<String>,
) -> Result<GroupModel, Errors>
where
    C: ConnectionTrait,
{
    let group = GroupActiveModel {
        name: Set(name),
        description: Set(description),
        is_system: Set(false),
        ..Default::default()
    };

    Ok(group.insert(conn).await?)
}
