use crate::repository::notification::{
    repository_create_notification_action_preference,
    repository_find_notification_action_preference,
    repository_update_notification_action_preference,
};
use crate::service::auth::session_types::SessionContext;
use constants::NotificationAction;
use entity::notification_action_preferences::Model as NotificationActionPreferenceModel;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};

/// Creates or updates the current user's per-action notification preference.
///
/// # Role
/// - Checks whether a record already exists.
/// - Performs an update if it does, otherwise a create.
///
/// # Related
/// - `repository_find_notification_action_preference`
/// - `repository_update_notification_action_preference`
/// - `repository_create_notification_action_preference`
///
/// # Transactions/Side effects
/// Wraps the read+write flow in a single transaction.
///
/// # Errors
/// - Returns a DB/repository error if the lookup or save fails.
pub async fn service_update_notification_action_preference(
    db: &DatabaseConnection,
    session: &SessionContext,
    action: NotificationAction,
    enabled: bool,
) -> ServiceResult<NotificationActionPreferenceModel> {
    let txn = db.begin().await?;

    // Check if preference exists
    let existing =
        repository_find_notification_action_preference(&txn, session.user_id, action).await?;

    let preference = if existing.is_some() {
        // Update existing preference
        repository_update_notification_action_preference(&txn, session.user_id, action, enabled)
            .await?
    } else {
        // Create new preference
        repository_create_notification_action_preference(&txn, session.user_id, action, enabled)
            .await?
    };

    txn.commit().await?;

    Ok(preference)
}
