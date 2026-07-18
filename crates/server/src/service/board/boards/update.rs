use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::repository_update_board;
use crate::service::auth::session_types::SessionContext;
use dto::board::{UpdateBoardRequest, UpdateBoardResponse};
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;

pub async fn service_update_board(
    db: &DatabaseConnection,
    payload: UpdateBoardRequest,
    session: &SessionContext,
) -> ServiceResult<UpdateBoardResponse> {
    let ctx = PermissionService::get_context(db, Some(session)).await?;
    BoardPermission::ManageBoard.check(&ctx)?;

    let txn = db.begin().await?;

    let board = repository_update_board(
        &txn,
        payload.board_id,
        payload.slug,
        payload.name,
        payload.description,
        payload.order,
        payload.is_disabled,
    )
    .await?;

    txn.commit().await?;

    info!(board_id = %board.id, "Board updated");

    Ok(UpdateBoardResponse { id: board.id })
}
