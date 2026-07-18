use chrono::{DateTime, Utc};
use entity::user_bans::{ActiveModel, Model};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use uuid::Uuid;

pub async fn repository_create_user_ban<C>(
    conn: &C,
    user_id: Uuid,
    reason: Option<String>,
    created_by: Option<Uuid>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Model, Errors>
where
    C: ConnectionTrait,
{
    let new_ban = ActiveModel {
        id: Default::default(),
        user_id: Set(user_id),
        reason: Set(reason),
        created_by: Set(created_by),
        expires_at: Set(expires_at),
        created_at: Default::default(),
    };

    let result = new_ban.insert(conn).await?;
    Ok(result)
}
