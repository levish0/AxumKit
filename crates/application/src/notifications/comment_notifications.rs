use super::{
    NotificationService,
    entities::{notification, push_delivery, push_device},
    push::{FCM_DELIVERY_JOB, PushJob},
    repository,
};
use crate::{
    ApplicationResult,
    database::database_time,
    identity::entities::account,
    notifications::PushDeliveryState,
    organizations::entities::membership,
    posts::entities::{comment, post},
};
use jobs::TraceContext;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::{LockType, OnConflict},
};
use uuid::Uuid;

impl NotificationService {
    pub(crate) async fn notify_comment_in_transaction(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        post: &post::Model,
        comment: &comment::Model,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        if comment.author_id == post.author_id {
            return Ok(());
        }
        let recipient = account::Entity::find_by_id(post.author_id)
            .filter(account::Column::Active.eq(true))
            .filter(account::Column::Verified.eq(true))
            .one(transaction)
            .await?;
        let member = membership::Entity::find()
            .filter(membership::Column::OrganizationId.eq(post.organization_id))
            .filter(membership::Column::AccountId.eq(post.author_id))
            .one(transaction)
            .await?;
        if recipient.is_none() || member.is_none() {
            return Ok(());
        }
        let preferences = repository::preferences(transaction, post.author_id).await?;
        if !preferences.post_comments {
            return Ok(());
        }
        let now = database_time(transaction).await?;
        let notification_id = Uuid::now_v7();
        let inserted = notification::Entity::insert(notification::ActiveModel {
            id: Set(notification_id),
            recipient_id: Set(post.author_id),
            actor_id: Set(comment.author_id),
            organization_id: Set(post.organization_id),
            post_id: Set(post.id),
            comment_id: Set(comment.id),
            created_at: Set(now),
            read_at: Set(None),
        })
        .on_conflict(
            OnConflict::columns([
                notification::Column::CommentId,
                notification::Column::RecipientId,
            ])
            .do_nothing()
            .to_owned(),
        )
        .exec_without_returning(transaction)
        .await?;
        if inserted == 0 || !preferences.push_enabled || self.fcm.is_none() {
            return Ok(());
        }
        let devices = push_device::Entity::find()
            .filter(push_device::Column::AccountId.eq(post.author_id))
            .filter(push_device::Column::ExpiresAt.gt(now))
            .order_by_asc(push_device::Column::Id)
            .limit(20)
            .lock(LockType::NoKeyUpdate)
            .all(transaction)
            .await?;
        for device in devices {
            let delivery = push_delivery::ActiveModel {
                id: Set(Uuid::now_v7()),
                notification_id: Set(notification_id),
                device_id: Set(device.id),
                device_version: Set(device.version),
                state: Set(PushDeliveryState::Pending),
                created_at: Set(now),
            }
            .insert(transaction)
            .await?;
            jobs::enqueue(
                transaction,
                Uuid::now_v7(),
                FCM_DELIVERY_JOB,
                &PushJob {
                    delivery_id: delivery.id,
                },
                trace,
            )
            .await?;
        }
        Ok(())
    }
}
