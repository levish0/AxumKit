use super::{
    NotificationService,
    entities::{notification, push_delivery, push_device},
    repository,
};
use crate::{
    ApplicationResult,
    authorization::{Action, PostFacts},
    database::database_time,
    identity::{
        entities::{account, session},
        service::principal,
    },
    notifications::PushDeliveryState,
    organizations::entities::organization,
    posts::{PostState, entities::post},
};
use jobs::{Claim, HandlerFailure};
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
    sea_query::{Expr, LockType},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FCM_DELIVERY_JOB: &str = "notifications.fcm";
#[derive(Serialize, Deserialize)]
pub(super) struct PushJob {
    pub delivery_id: Uuid,
}
struct PreparedPush {
    device_id: Uuid,
    device_version: i64,
    token: String,
    notification_id: Uuid,
}

impl NotificationService {
    #[tracing::instrument(skip_all, fields(job_id = %claim.id, request_id = %claim.request_id))]
    pub async fn execute_job(&self, claim: &Claim) -> Result<(), HandlerFailure> {
        if claim.kind != FCM_DELIVERY_JOB {
            return Err(HandlerFailure {
                code: "unknown_notification_job",
                retryable: false,
            });
        }
        let payload: PushJob =
            serde_json::from_value(claim.payload.clone()).map_err(|_| HandlerFailure {
                code: "invalid_push_payload",
                retryable: false,
            })?;
        let fcm = self.fcm.as_ref().ok_or(HandlerFailure {
            code: "fcm_disabled",
            retryable: false,
        })?;
        let prepared =
            self.prepare_push(payload.delivery_id)
                .await
                .map_err(|_| HandlerFailure {
                    code: "push_preparation_failed",
                    retryable: true,
                })?;
        let Some(prepared) = prepared else {
            self.finish_push(payload.delivery_id, PushDeliveryState::Skipped)
                .await
                .map_err(|_| HandlerFailure {
                    code: "push_storage_failed",
                    retryable: true,
                })?;
            return Ok(());
        };
        match fcm
            .send(
                &prepared.token,
                prepared.notification_id,
                payload.delivery_id,
            )
            .await
        {
            Ok(()) => self
                .finish_push(payload.delivery_id, PushDeliveryState::Sent)
                .await
                .map_err(|_| HandlerFailure {
                    code: "push_storage_failed",
                    retryable: true,
                }),
            Err(failure) if failure.unregistered => {
                push_device::Entity::delete_many()
                    .filter(push_device::Column::Id.eq(prepared.device_id))
                    .filter(push_device::Column::Version.eq(prepared.device_version))
                    .exec(&self.identity.database)
                    .await
                    .map_err(|_| HandlerFailure {
                        code: "push_storage_failed",
                        retryable: true,
                    })?;
                Ok(())
            }
            Err(failure) => {
                if let Some(seconds) = failure.retry_after_seconds {
                    jobs::PostgresQueue::new(self.identity.database.clone())
                        .defer_retry(claim, seconds)
                        .await
                        .map_err(|_| HandlerFailure {
                            code: "push_retry_schedule_failed",
                            retryable: true,
                        })?;
                }
                Err(HandlerFailure {
                    code: failure.code,
                    retryable: failure.retryable,
                })
            }
        }
    }

    async fn finish_push(
        &self,
        delivery_id: Uuid,
        state: PushDeliveryState,
    ) -> ApplicationResult<()> {
        push_delivery::Entity::update_many()
            .col_expr(push_delivery::Column::State, Expr::value(state))
            .filter(push_delivery::Column::Id.eq(delivery_id))
            .filter(push_delivery::Column::State.eq(PushDeliveryState::Pending))
            .exec(&self.identity.database)
            .await?;
        Ok(())
    }

    async fn prepare_push(&self, delivery_id: Uuid) -> ApplicationResult<Option<PreparedPush>> {
        let Some(delivery) = push_delivery::Entity::find_by_id(delivery_id)
            .filter(push_delivery::Column::State.eq(PushDeliveryState::Pending))
            .one(&self.identity.database)
            .await?
        else {
            return Ok(None);
        };
        let Some(snapshot) = notification::Entity::find_by_id(delivery.notification_id)
            .one(&self.identity.database)
            .await?
        else {
            return Ok(None);
        };
        let transaction = self.identity.database.begin().await?;
        let Some(recipient) = account::Entity::find_by_id(snapshot.recipient_id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
            .filter(|account| {
                account.active
                    && account.verified
                    && (!account.administrator || account.totp_secret.is_some())
            })
        else {
            return Ok(None);
        };
        let Some(organization) = organization::Entity::find_by_id(snapshot.organization_id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
        else {
            return Ok(None);
        };
        let Some(notification) = repository::visible(recipient.id)
            .filter(notification::Column::Id.eq(snapshot.id))
            .filter(notification::Column::ReadAt.is_null())
            .one(&transaction)
            .await?
        else {
            return Ok(None);
        };
        let post = post::Entity::find_by_id(notification.post_id)
            .one(&transaction)
            .await?
            .ok_or(crate::ApplicationError::NotFound)?;
        self.identity.authorization.require_post(
            principal(&recipient),
            Action::ReadPost,
            PostFacts {
                id: post.id,
                organization_id: post.organization_id,
                organization_owner_id: organization.owner_id,
                author_id: post.author_id,
                published: post.state == PostState::Published,
                deleted: post.state == PostState::Deleted,
                organization_member: true,
            },
        )?;
        let preferences = repository::preferences(&transaction, recipient.id).await?;
        if !preferences.post_comments || !preferences.push_enabled {
            return Ok(None);
        }
        let Some(device) = push_device::Entity::find_by_id(delivery.device_id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
            .filter(|device| {
                device.account_id == recipient.id && device.version == delivery.device_version
            })
        else {
            return Ok(None);
        };
        let now = database_time(&transaction).await?;
        if device.expires_at <= now {
            return Ok(None);
        }
        let session = session::Entity::find_by_id(device.session_id)
            .filter(session::Column::AccountId.eq(recipient.id))
            .filter(session::Column::SecurityVersion.eq(recipient.security_version))
            .filter(session::Column::ExpiresAt.gt(now))
            .filter(session::Column::IdleExpiresAt.gt(now))
            .one(&transaction)
            .await?;
        if session.is_none() {
            return Ok(None);
        }
        if push_delivery::Entity::find_by_id(delivery_id)
            .filter(push_delivery::Column::State.eq(PushDeliveryState::Pending))
            .one(&transaction)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let token = self.identity.settings.secret_keys.decrypt(
            &format!("fcm-device:{}", device.id),
            &device.encrypted_token,
        )?;
        let token = String::from_utf8(token).map_err(crate::ApplicationError::internal)?;
        transaction.commit().await?;
        Ok(Some(PreparedPush {
            device_id: device.id,
            device_version: device.version,
            token,
            notification_id: notification.id,
        }))
    }
}
