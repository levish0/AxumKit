use crate::repository::notification::repository_delete_notification;
use crate::service::auth::session_types::SessionContext;
use errors::errors::{Errors, ServiceResult};
use sea_orm::DatabaseConnection;
use tracing::debug;
use uuid::Uuid;

/// Deletes a single notification belonging to the currently logged-in user.
///
/// # Role
/// - Only deletes notifications owned by the user.
/// - Returns NotFound if there is nothing to delete.
///
/// # Related
/// - `repository_delete_notification`
///
/// # Errors
/// - `Errors::NotFound` if the target does not exist
/// - Returns a DB/repository error if the deletion fails.
pub async fn service_delete_notification(
    db: &DatabaseConnection,
    session: &SessionContext,
    notification_id: Uuid,
) -> ServiceResult<()> {
    let rows_affected =
        repository_delete_notification(db, notification_id, session.user_id).await?;

    if rows_affected == 0 {
        return Err(Errors::NotFound(
            "Notification not found or not owned by user".to_string(),
        ));
    }

    debug!(user_id = %session.user_id, notification_id = %notification_id, "Notification deleted");

    Ok(())
}
