use entity::boards::{ActiveModel as BoardActiveModel, Model as BoardModel};
use errors::errors::Errors;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

pub async fn repository_create_board<C>(
    conn: &C,
    slug: String,
    name: String,
    description: Option<String>,
    order: i32,
) -> Result<BoardModel, Errors>
where
    C: ConnectionTrait,
{
    let new_board = BoardActiveModel {
        id: Default::default(),
        slug: Set(slug),
        name: Set(name),
        description: Set(description),
        order: Set(order),
        is_disabled: Set(false),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let board = new_board.insert(conn).await?;
    Ok(board)
}
