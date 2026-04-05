use axumkit_entity::common::Role;
use axumkit_entity::user_roles::{ActiveModel, Model};
use axumkit_errors::errors::Errors;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_user_role<C>(
    conn: &C,
    user_id: Uuid,
    role: Role,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Model, Errors>
where
    C: ConnectionTrait,
{
    let new_role = ActiveModel {
        id: Default::default(),
        user_id: Set(user_id),
        role: Set(role),
        granted_at: Default::default(),
        expires_at: Set(expires_at),
    };

    let result = new_role.insert(conn).await?;
    Ok(result)
}
