use crate::permission::PermissionService;
use crate::permission::board::BoardPermission;
use crate::permission::rule::Rule;
use crate::repository::board::repository_create_board;
use crate::service::auth::session_types::SessionContext;
use dto::board::{CreateBoardRequest, CreateBoardResponse};
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;

pub async fn service_create_board(
    db: &DatabaseConnection,
    payload: CreateBoardRequest,
    session: &SessionContext,
) -> ServiceResult<CreateBoardResponse> {
    let ctx = PermissionService::get_context(db, Some(session)).await?;
    BoardPermission::ManageBoard.check(&ctx)?;

    let txn = db.begin().await?;

    let board = repository_create_board(
        &txn,
        payload.slug,
        payload.name,
        payload.description,
        payload.order.unwrap_or(0),
    )
    .await?;

    txn.commit().await?;

    info!(board_id = %board.id, "Board created");

    Ok(CreateBoardResponse { id: board.id })
}
