use crate::permission::board::BoardFacts;
use entity::boards::Model as BoardModel;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;

/// Loads the permission facts for a board.
pub async fn load_board_facts<C>(_conn: &C, board: &BoardModel) -> Result<BoardFacts, Errors>
where
    C: ConnectionTrait,
{
    Ok(BoardFacts {
        is_disabled: board.is_disabled,
    })
}

/// Loads facts for many boards, preserving order.
pub async fn load_board_facts_batch<C>(
    _conn: &C,
    boards: Vec<BoardModel>,
) -> Result<Vec<(BoardModel, BoardFacts)>, Errors>
where
    C: ConnectionTrait,
{
    Ok(boards
        .into_iter()
        .map(|board| {
            let facts = BoardFacts {
                is_disabled: board.is_disabled,
            };
            (board, facts)
        })
        .collect())
}
