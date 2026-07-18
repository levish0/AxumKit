use crate::repository::notification::repository_mark_all_notifications_as_read;
use crate::service::auth::session_types::SessionContext;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use tracing::debug;

/// Marks all unread notifications of the currently logged-in user as read.
///
/// # Role
/// Bulk-updates unread notifications and returns the number of affected rows.
///
/// # Related
/// - `repository_mark_all_notifications_as_read`
///
/// # Errors
/// - Returns a DB/repository error if the update fails.
pub async fn service_mark_all_notifications_as_read(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<u64> {
    let result = repository_mark_all_notifications_as_read(db, session.user_id).await?;

    debug!(user_id = %session.user_id, count = result.rows_affected, "Notifications marked as read");

    Ok(result.rows_affected)
}
