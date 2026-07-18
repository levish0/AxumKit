use crate::repository::notification::{
    NotificationFilter, repository_exists_newer_notification, repository_exists_older_notification,
    repository_find_notifications_by_user_id_cursor,
};
use crate::service::actors::actor_response_map;
use crate::service::auth::session_types::SessionContext;
use crate::service::cursor_pagination::{cursor_flags, reverse_if_newer};
use constants::NotificationAction;
use dto::notification::{GetNotificationsRequest, NotificationListResponse, NotificationResponse};
use dto::pagination::CursorDirection;
use errors::errors::ServiceResult;
use sea_orm::DatabaseConnection;
use std::collections::HashSet;
use std::str::FromStr;
use tracing::warn;

/// Retrieves the currently logged-in user's notifications with cursor-based pagination.
///
/// # Role
/// - Converts request conditions into a `NotificationFilter`.
/// - Computes the cursor page and `has_newer`/`has_older`.
/// - Maps repository models to API response DTOs.
///
/// # Related
/// - `repository_find_notifications_by_user_id_cursor`
/// - `repository_exists_newer_notification`
/// - `repository_exists_older_notification`
/// - `crate::service::cursor_pagination`
///
/// # Errors
/// - Returns a DB/repository error if the query fails.
pub async fn service_get_notifications(
    db: &DatabaseConnection,
    session: &SessionContext,
    payload: GetNotificationsRequest,
) -> ServiceResult<NotificationListResponse> {
    let limit = payload.limit;
    let is_newer = payload.cursor_direction == Some(CursorDirection::Newer);

    // Build filter from request
    let filter = NotificationFilter {
        notification_type: payload.notification_type,
        actions: payload.actions,
        is_read: payload.is_read,
        board_id: payload.board_id,
        post_id: payload.post_id,
    };

    let mut notifications = repository_find_notifications_by_user_id_cursor(
        db,
        session.user_id,
        payload.cursor_notification_id,
        payload.cursor_direction,
        &filter,
        limit,
    )
    .await?;

    let (has_newer, has_older) = cursor_flags(
        &notifications,
        is_newer,
        |notification| notification.id,
        |cursor| repository_exists_newer_notification(db, session.user_id, &filter, cursor),
        |cursor| repository_exists_older_notification(db, session.user_id, &filter, cursor),
    )
    .await?;

    reverse_if_newer(&mut notifications, is_newer);

    let actor_ids: Vec<_> = notifications
        .iter()
        .filter_map(|notification| notification.actor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let actors = actor_response_map(db, &actor_ids).await?;

    let data: Vec<NotificationResponse> = notifications
        .into_iter()
        .filter_map(|notification| {
            let action = match NotificationAction::from_str(&notification.action) {
                Ok(a) => a,
                Err(_) => {
                    warn!(
                        notification_id = %notification.id,
                        action = %notification.action,
                        "Failed to parse notification action, skipping"
                    );
                    return None;
                }
            };
            Some(NotificationResponse {
                id: notification.id,
                actor_id: notification.actor_id,
                actor: notification
                    .actor_id
                    .and_then(|id| actors.get(&id).cloned()),
                notification_type: notification.notification_type,
                action,
                board_id: notification.board_id,
                post_id: notification.post_id,
                comment_id: notification.comment_id,
                additional_data: notification.additional_data,
                is_read: notification.is_read,
                created_at: notification.created_at,
                read_at: notification.read_at,
            })
        })
        .collect();

    Ok(NotificationListResponse {
        data,
        has_newer,
        has_older,
    })
}
