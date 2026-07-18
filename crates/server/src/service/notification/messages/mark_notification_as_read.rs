use crate::repository::notification::repository_mark_notification_as_read;
use crate::service::auth::session_types::SessionContext;
use errors::errors::{Errors, ServiceResult};
use sea_orm::DatabaseConnection;
use tracing::debug;
use uuid::Uuid;

/// Marks a single notification of the currently logged-in user as read.
///
/// # Role
/// - Transitions the notification to read state with a single UPDATE.
/// - Returns NotFound if the target does not exist or is already read.
///
/// # Related
/// - `repository_mark_notification_as_read`
///
/// # Errors
/// - `Errors::NotFound` if the target does not exist
/// - Returns a DB/repository error if the update fails.
pub async fn service_mark_notification_as_read(
    db: &DatabaseConnection,
    session: &SessionContext,
    notification_id: Uuid,
) -> ServiceResult<()> {
    // Single atomic UPDATE, no transaction needed
    let rows_affected =
        repository_mark_notification_as_read(db, notification_id, session.user_id).await?;

    if rows_affected == 0 {
        return Err(Errors::NotFound(
            "Notification not found or already read".to_string(),
        ));
    }

    debug!(user_id = %session.user_id, notification_id = %notification_id, "Notification marked as read");

    Ok(())
}
