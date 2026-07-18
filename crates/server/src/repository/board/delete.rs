use entity::boards::Entity as BoardEntity;
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, ModelTrait};
use uuid::Uuid;

pub async fn repository_delete_board<C>(conn: &C, id: Uuid) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let board = BoardEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardNotFound)?;

    board.delete(conn).await?;
    Ok(())
}
