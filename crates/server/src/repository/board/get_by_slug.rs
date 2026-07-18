use entity::boards::{Column, Entity as BoardEntity, Model as BoardModel};
use errors::errors::Errors;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn repository_get_board_by_slug<C>(conn: &C, slug: &str) -> Result<BoardModel, Errors>
where
    C: ConnectionTrait,
{
    let board = BoardEntity::find()
        .filter(Column::Slug.eq(slug))
        .one(conn)
        .await?
        .ok_or(Errors::BoardNotFound)?;

    Ok(board)
}
