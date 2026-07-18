use super::get::authorize_view_and_map;
use crate::repository::board::repository_get_board_by_slug;
use crate::service::auth::session_types::SessionContext;
use dto::board::BoardResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

pub async fn service_get_board_by_slug(
    db: &DatabaseConnection,
    slug: &str,
    session: Option<&SessionContext>,
) -> ServiceResult<BoardResponse> {
    let board = repository_get_board_by_slug(db, slug).await?;
    authorize_view_and_map(db, board, session).await
}
