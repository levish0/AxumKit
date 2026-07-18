use chrono::Utc;
use entity::boards::{ActiveModel as BoardActiveModel, Entity as BoardEntity, Model as BoardModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use uuid::Uuid;

pub async fn repository_update_board<C>(
    conn: &C,
    id: Uuid,
    slug: Option<String>,
    name: Option<String>,
    description: Option<Option<String>>,
    order: Option<i32>,
    is_disabled: Option<bool>,
) -> Result<BoardModel, Errors>
where
    C: ConnectionTrait,
{
    let board = BoardEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardNotFound)?;

    let mut active: BoardActiveModel = board.into();

    if let Some(slug) = slug {
        active.slug = Set(slug);
    }
    if let Some(name) = name {
        active.name = Set(name);
    }
    if let Some(description) = description {
        active.description = Set(description);
    }
    if let Some(order) = order {
        active.order = Set(order);
    }
    if let Some(is_disabled) = is_disabled {
        active.is_disabled = Set(is_disabled);
    }
    active.updated_at = Set(Utc::now());

    let updated = active.update(conn).await?;
    Ok(updated)
}
