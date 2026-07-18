use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use dto::user::CheckHandleAvailableResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// Checks whether a user handle is available.
///
/// # Role
/// Checks for an existing user with the same handle and returns an `available` flag.
///
/// # Related
/// - `repository_find_user_by_handle`
///
/// # Errors
/// - Returns a DB/repository error if the lookup fails.
pub async fn service_check_handle_available(
    db: &DatabaseConnection,
    handle: &str,
) -> ServiceResult<CheckHandleAvailableResponse> {
    let user = repository_find_user_by_handle(db, handle.to_string()).await?;

    Ok(CheckHandleAvailableResponse {
        available: user.is_none(),
    })
}
