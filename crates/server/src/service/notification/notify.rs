use crate::repository::notification::{
    NotificationTarget, repository_create_notification,
    repository_find_notification_action_preference,
};
use constants::NotificationAction;
use entity::common::NotificationType;
use errors::errors::Errors;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::IpNetwork;
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// The single server-side entry point for creating a directly-targeted
/// notification (mentions, discussion assignment, edit/file-replace review
/// results). It honors the recipient's per-action preference before creating
/// anything — an opt-out model where an absent row means enabled.
///
/// The worker watcher fan-out applies the same preference filter, so routing
/// every direct notification through here keeps "may this user receive this
/// action" defined in one place: previously the direct call sites wrote the
/// notification unconditionally, so disabling e.g. `user_mentioned` had no
/// effect on mentions created on the server.
///
/// Returns `Ok(false)` when the recipient has the action disabled (nothing
/// created), `Ok(true)` when a notification was created.
pub async fn service_notify_user(
    db: &DatabaseConnection,
    recipient_user_id: Uuid,
    actor_id: Option<Uuid>,
    actor_ip: Option<IpNetwork>,
    notification_type: NotificationType,
    action: NotificationAction,
    target: NotificationTarget,
    additional_data: JsonValue,
) -> Result<bool, Errors> {
    let disabled = repository_find_notification_action_preference(db, recipient_user_id, action)
        .await?
        .is_some_and(|preference| !preference.enabled);

    if disabled {
        return Ok(false);
    }

    repository_create_notification(
        db,
        recipient_user_id,
        actor_id,
        actor_ip,
        notification_type,
        action,
        target,
        additional_data,
    )
    .await?;

    Ok(true)
}

/// Notifies each mentioned user of a `user_mentioned` event, skipping the author
/// (`exclude_user_id`). Consolidates the mention → notification loop that was
/// copy-pasted across the board post/comment and discussion message create and
/// update paths. Best-effort per recipient (a failure is dropped, not
/// propagated) and preference-checked via `service_notify_user`.
pub async fn notify_mentions(
    db: &DatabaseConnection,
    mentioned_user_ids: impl IntoIterator<Item = Uuid>,
    exclude_user_id: Option<Uuid>,
    actor_id: Uuid,
    actor_ip: Option<IpNetwork>,
    target: NotificationTarget,
    additional_data: JsonValue,
) {
    for user_id in mentioned_user_ids {
        if Some(user_id) == exclude_user_id {
            continue;
        }
        let _ = service_notify_user(
            db,
            user_id,
            Some(actor_id),
            actor_ip,
            NotificationType::User,
            NotificationAction::UserMentioned,
            target.clone(),
            additional_data.clone(),
        )
        .await;
    }
}
