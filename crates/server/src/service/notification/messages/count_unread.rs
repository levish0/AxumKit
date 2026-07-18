use crate::repository::notification::repository_count_unread_notifications;
use crate::service::auth::session_types::SessionContext;
use dto::notification::UnreadCountResponse;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;

/// Retrieves the unread notification count for the currently logged-in user.
///
/// # Role
/// Computes the user's unread notification count and returns it as a response DTO.
///
/// # Related
/// - `repository_count_unread_notifications`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn service_count_unread_notifications(
    db: &DatabaseConnection,
    session: &SessionContext,
) -> ServiceResult<UnreadCountResponse> {
    let count = repository_count_unread_notifications(db, session.user_id).await?;

    Ok(UnreadCountResponse { count })
}
