use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::repository_find_boards;
use crate::service::auth::session_types::SessionContext;
use crate::service::board::facts::load_board_facts_batch;
use crate::service::board::mapper::build_board_response;
use dto::board::{BoardListResponse, BoardResponse, GetBoardsRequest};
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

pub async fn service_list_boards(
    db: &DatabaseConnection,
    payload: GetBoardsRequest,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardListResponse> {
    // Listing is a read: bans are exempt (reads are never ban-gated). The
    // View filter below still hides disabled/restricted boards from everyone.
    let ctx = PermissionService::get_context(db, session).await?;

    let boards = repository_find_boards(db).await?;

    // Resolve every board's facts in one batched query, then keep only the ones
    // the caller may view (retaining facts so the mapper reuses the chain).
    let visible_boards: Vec<_> = load_board_facts_batch(db, boards)
        .await?
        .into_iter()
        .filter(|(_, facts)| BoardPermission::View(facts.clone()).is_allowed(&ctx))
        .collect();

    let page = payload.page;
    let offset = (page as usize - 1) * payload.page_size as usize;
    let page_size = payload.page_size as usize;
    let has_more = visible_boards.len() > offset + page_size;

    let board_responses: Vec<BoardResponse> = visible_boards
        .into_iter()
        .skip(offset)
        .take(page_size)
        .map(|(board, facts)| build_board_response(&ctx, board, facts))
        .collect();

    Ok(BoardListResponse {
        boards: board_responses,
        current_page: page,
        page_size: page_size as u32,
        has_more,
    })
}
