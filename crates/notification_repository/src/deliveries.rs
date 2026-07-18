//! Notification event + delivery insertion: the single definition of how a
//! `notification_events` row and its per-recipient `notification_deliveries`
//! rows are written.
//!
//! Both the API server (direct, single-recipient notifications such as mentions
//! and comment alerts) and the worker (batched fan-out) go through
//! [`insert_notification_event_deliveries`], so the event's column set,
//! the action encoding and the delivery defaults are defined once instead of
//! being mirrored field-for-field across the two crates.

use constants::NotificationAction;
use entity::common::{NotificationTargetKind, NotificationType};
use entity::notification_deliveries::{
    ActiveModel as NotificationDeliveryActiveModel, Entity as NotificationDeliveryEntity,
};
use entity::notification_events::ActiveModel as NotificationEventActiveModel;
use sea_orm::prelude::{IpNetwork, Json};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait, Set};
use uuid::Uuid;

/// The event's target: which entity a notification points at. Converted to the
/// mutually-exclusive `notification_events` FK columns by [`into_columns`].
///
/// [`into_columns`]: NotificationTarget::into_columns
#[derive(Debug, Clone)]
pub enum NotificationTarget {
    None,
    BoardPost {
        board_id: Uuid,
        post_id: Uuid,
    },
    BoardComment {
        board_id: Uuid,
        post_id: Uuid,
        comment_id: Uuid,
    },
}

/// The `notification_events` target columns produced from a [`NotificationTarget`].
#[derive(Debug, Clone, PartialEq, Eq)]
struct NotificationEventColumns {
    target_kind: NotificationTargetKind,
    board_id: Option<Uuid>,
    post_id: Option<Uuid>,
    comment_id: Option<Uuid>,
}

impl NotificationTarget {
    fn into_columns(self) -> NotificationEventColumns {
        // Start from all-None and set only the columns this target uses, so the
        // mutual exclusivity between target kinds is enforced in one place.
        let base = NotificationEventColumns {
            target_kind: NotificationTargetKind::None,
            board_id: None,
            post_id: None,
            comment_id: None,
        };

        match self {
            Self::None => base,
            Self::BoardPost { board_id, post_id } => NotificationEventColumns {
                target_kind: NotificationTargetKind::BoardPost,
                board_id: Some(board_id),
                post_id: Some(post_id),
                ..base
            },
            Self::BoardComment {
                board_id,
                post_id,
                comment_id,
            } => NotificationEventColumns {
                target_kind: NotificationTargetKind::BoardComment,
                board_id: Some(board_id),
                post_id: Some(post_id),
                comment_id: Some(comment_id),
            },
        }
    }
}

/// Everything needed to write one notification event (shared by all recipients).
#[derive(Debug, Clone)]
pub struct NotificationEventInsertSpec {
    pub actor_id: Option<Uuid>,
    pub actor_ip: Option<IpNetwork>,
    pub notification_type: NotificationType,
    pub action: NotificationAction,
    pub target: NotificationTarget,
    pub additional_data: Json,
}

/// Insert one `notification_events` row and one `notification_deliveries` row per
/// recipient, on the given connection. Returns the number of deliveries created
/// (0 for no recipients).
///
/// The caller owns the transaction: pass a `&DatabaseTransaction` (or otherwise
/// atomic connection) and commit it, so the event and every delivery batch
/// commit together — a partial write would let a job retry duplicate
/// notifications for already-inserted recipients.
pub async fn insert_notification_event_deliveries<C: ConnectionTrait>(
    conn: &C,
    recipients: &[Uuid],
    spec: NotificationEventInsertSpec,
    batch_size: usize,
) -> Result<usize, DbErr> {
    if recipients.is_empty() {
        return Ok(0);
    }

    let columns = spec.target.into_columns();

    let event = NotificationEventActiveModel {
        id: Default::default(),
        actor_id: Set(spec.actor_id),
        actor_ip: Set(spec.actor_ip),
        notification_type: Set(spec.notification_type),
        action: Set(spec.action.to_string()),
        target_kind: Set(columns.target_kind),
        board_id: Set(columns.board_id),
        post_id: Set(columns.post_id),
        comment_id: Set(columns.comment_id),
        additional_data: Set(Some(spec.additional_data)),
        created_at: Default::default(),
    }
    .insert(conn)
    .await?;

    let mut created = 0usize;
    for batch in recipients.chunks(batch_size.max(1)) {
        let models: Vec<NotificationDeliveryActiveModel> = batch
            .iter()
            .map(|user_id| NotificationDeliveryActiveModel {
                id: Default::default(),
                user_id: Set(*user_id),
                event_id: Set(event.id),
                is_read: Set(false),
                created_at: Default::default(),
                read_at: Set(None),
            })
            .collect();

        NotificationDeliveryEntity::insert_many(models)
            .exec(conn)
            .await?;

        created += batch.len();
    }

    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::NotificationTarget;
    use entity::common::NotificationTargetKind;
    use uuid::Uuid;

    #[test]
    fn board_post_target_sets_only_board_columns() {
        let board_id = Uuid::now_v7();
        let post_id = Uuid::now_v7();

        let columns = NotificationTarget::BoardPost { board_id, post_id }.into_columns();

        assert_eq!(columns.target_kind, NotificationTargetKind::BoardPost);
        assert_eq!(columns.board_id, Some(board_id));
        assert_eq!(columns.post_id, Some(post_id));
        assert_eq!(columns.comment_id, None);
    }

    #[test]
    fn board_comment_target_sets_board_and_comment_columns() {
        let board_id = Uuid::now_v7();
        let post_id = Uuid::now_v7();
        let comment_id = Uuid::now_v7();

        let columns = NotificationTarget::BoardComment {
            board_id,
            post_id,
            comment_id,
        }
        .into_columns();

        assert_eq!(columns.target_kind, NotificationTargetKind::BoardComment);
        assert_eq!(columns.board_id, Some(board_id));
        assert_eq!(columns.post_id, Some(post_id));
        assert_eq!(columns.comment_id, Some(comment_id));
    }

    #[test]
    fn none_target_sets_no_columns() {
        let columns = NotificationTarget::None.into_columns();

        assert_eq!(columns.target_kind, NotificationTargetKind::None);
        assert_eq!(columns.board_id, None);
        assert_eq!(columns.post_id, None);
        assert_eq!(columns.comment_id, None);
    }
}
