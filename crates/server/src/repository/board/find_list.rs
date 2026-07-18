use entity::boards::{Column as BoardColumn, Entity as BoardEntity, Model as BoardModel};
use errors::errors::Errors;
use sea_orm::{ConnectionTrait, EntityTrait, QueryOrder};

pub async fn repository_find_boards<C>(conn: &C) -> Result<Vec<BoardModel>, Errors>
where
    C: ConnectionTrait,
{
    let boards = BoardEntity::find()
        .order_by_asc(BoardColumn::Order)
        .order_by_asc(BoardColumn::Id)
        .all(conn)
        .await?;

    Ok(boards)
}
