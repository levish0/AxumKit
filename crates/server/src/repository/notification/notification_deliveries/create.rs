use super::NotificationTarget;
use constants::NotificationAction;
use entity::common::NotificationType;
use errors::errors::Errors;
use notification_repository::{NotificationEventInsertSpec, insert_notification_event_deliveries};
use sea_orm::prelude::IpNetwork;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Creates notification event/delivery records for a single recipient.
///
/// # Role
/// Stores the event body in `notification_events` and the per-user read state
/// separately in `notification_deliveries`. The actual insert is handled by
/// `notification_repository::insert_notification_event_deliveries`, shared
/// between the server and the worker.
///
/// Since this is called on paths after the main transaction has committed, it
/// opens a short internal transaction so the event + delivery inserts are atomic.
pub async fn repository_create_notification(
    db: &DatabaseConnection,
    user_id: Uuid,
    actor_id: Option<Uuid>,
    actor_ip: Option<IpNetwork>,
    notification_type: NotificationType,
    action: NotificationAction,
    target: NotificationTarget,
    additional_data: JsonValue,
) -> Result<(), Errors> {
    let txn = db.begin().await?;

    insert_notification_event_deliveries(
        &txn,
        &[user_id],
        NotificationEventInsertSpec {
            actor_id,
            actor_ip,
            notification_type,
            action,
            target,
            additional_data,
        },
        1,
    )
    .await?;

    txn.commit().await?;

    Ok(())
}
