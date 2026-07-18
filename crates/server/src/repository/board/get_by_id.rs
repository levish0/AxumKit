use entity::boards::{Entity as BoardEntity, Model as BoardModel};
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait};
use uuid::Uuid;

pub async fn repository_get_board_by_id<C>(conn: &C, id: Uuid) -> Result<BoardModel, Errors>
where
    C: ConnectionTrait,
{
    let board = BoardEntity::find_by_id(id)
        .one(conn)
        .await?
        .ok_or(Errors::BoardNotFound)?;

    Ok(board)
}
