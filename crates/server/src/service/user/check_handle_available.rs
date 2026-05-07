use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use dto::user::CheckHandleAvailableResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

pub async fn service_check_handle_available(
    conn: &DatabaseConnection,
    handle: &str,
) -> ServiceResult<CheckHandleAvailableResponse> {
    let user = repository_find_user_by_handle(conn, handle.to_string()).await?;

    Ok(CheckHandleAvailableResponse {
        available: user.is_none(),
    })
}
