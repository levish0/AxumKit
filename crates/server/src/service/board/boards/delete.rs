use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::repository_delete_board;
use crate::service::auth::session_types::SessionContext;
use dto::board::DeleteBoardResponse;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;

pub async fn service_delete_board(
    db: &DatabaseConnection,
    board_id: Uuid,
    session: &SessionContext,
) -> ServiceResult<DeleteBoardResponse> {
    let ctx = PermissionService::get_context(db, Some(session)).await?;
    BoardPermission::ManageBoard.check(&ctx)?;

    let txn = db.begin().await?;
    repository_delete_board(&txn, board_id).await?;
    txn.commit().await?;

    info!(board_id = %board_id, "Board deleted");

    Ok(DeleteBoardResponse { id: board_id })
}
