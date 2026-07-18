use crate::repository::notification::{
    repository_create_notification_preferences,
    repository_find_notification_preferences_by_user_id,
    repository_update_notification_preferences,
};
use crate::service::auth::session_types::SessionContext;
use dto::notification::NotificationPreferenceResponse;
use errors::errors::ServiceResult;
use sea_orm::{DatabaseConnection, TransactionTrait};

/// Creates or updates the current user's per-channel notification preferences.
///
/// # Role
/// - Checks whether a record already exists.
/// - Applies a partial update if it does, otherwise creates one with defaults filled in.
///
/// # Related
/// - `repository_find_notification_preferences_by_user_id`
/// - `repository_update_notification_preferences`
/// - `repository_create_notification_preferences`
///
/// # Transactions/Side effects
/// Wraps the read+write flow in a single transaction.
///
/// # Errors
/// - Returns a DB/repository error if the lookup or save fails.
pub async fn service_update_notification_preferences(
    db: &DatabaseConnection,
    session: &SessionContext,
    email_enabled: Option<bool>,
    push_enabled: Option<bool>,
) -> ServiceResult<NotificationPreferenceResponse> {
    let txn = db.begin().await?;

    // Check if preferences exist
    let existing =
        repository_find_notification_preferences_by_user_id(&txn, session.user_id).await?;

    let preferences = if existing.is_some() {
        // Update existing preferences
        repository_update_notification_preferences(
            &txn,
            session.user_id,
            email_enabled,
            push_enabled,
        )
        .await?
    } else {
        // Create new preferences (use provided values or defaults)
        repository_create_notification_preferences(
            &txn,
            session.user_id,
            email_enabled.unwrap_or(false), // Default: email enabled
            push_enabled.unwrap_or(false),  // Default: push disabled
        )
        .await?
    };

    txn.commit().await?;

    Ok(NotificationPreferenceResponse {
        email_enabled: preferences.email_enabled,
        push_enabled: preferences.push_enabled,
        updated_at: preferences.updated_at,
    })
}
