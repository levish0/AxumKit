use super::{NotificationService, PushDeviceSummary, entities::push_device, validation};
use crate::{
    ApplicationError, ApplicationResult, database::database_time, identity::AuthenticatedSession,
    notifications::PushPlatform,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::LockType,
};
use uuid::Uuid;

impl NotificationService {
    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id()))]
    pub async fn register_device(
        &self,
        actor: &AuthenticatedSession,
        token: &str,
        platform: PushPlatform,
    ) -> ApplicationResult<PushDeviceSummary> {
        if self.fcm.is_none() {
            return Err(ApplicationError::IntegrationUnavailable);
        }
        validation::device_token(token)?;
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let now = database_time(&transaction).await?;
        let token_hash = auth_core::token::hash_token(&format!("FCM registration v1:{token}"));
        let current = push_device::Entity::find()
            .filter(push_device::Column::TokenHash.eq(&token_hash))
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?;
        let count = push_device::Entity::find()
            .filter(push_device::Column::AccountId.eq(actor.account_id()))
            .filter(push_device::Column::ExpiresAt.gt(now))
            .count(&transaction)
            .await?;
        if count >= 20
            && !current
                .as_ref()
                .is_some_and(|row| row.account_id == actor.account_id() && row.expires_at > now)
        {
            return Err(ApplicationError::Conflict);
        }
        let expires_at = now + chrono::Duration::days(30);
        let row = match current {
            Some(row) => {
                let same_binding = row.account_id == actor.account_id()
                    && row.session_id == actor.session_id()
                    && row.platform == platform
                    && row.expires_at > now;
                let version = if same_binding {
                    row.version
                } else {
                    row.version
                        .checked_add(1)
                        .ok_or(ApplicationError::Conflict)?
                };
                let encrypted = self
                    .identity
                    .settings
                    .secret_keys
                    .encrypt(&format!("fcm-device:{}", row.id), token.as_bytes())?;
                let mut updated = row.into_active_model();
                updated.account_id = Set(actor.account_id());
                updated.session_id = Set(actor.session_id());
                updated.encrypted_token = Set(encrypted);
                updated.platform = Set(platform);
                updated.version = Set(version);
                updated.expires_at = Set(expires_at);
                updated.update(&transaction).await?
            }
            None => {
                let id = Uuid::now_v7();
                let encrypted = self
                    .identity
                    .settings
                    .secret_keys
                    .encrypt(&format!("fcm-device:{id}"), token.as_bytes())?;
                push_device::ActiveModel {
                    id: Set(id),
                    account_id: Set(actor.account_id()),
                    session_id: Set(actor.session_id()),
                    token_hash: Set(token_hash),
                    encrypted_token: Set(encrypted),
                    platform: Set(platform),
                    version: Set(1),
                    expires_at: Set(expires_at),
                    created_at: Set(now),
                }
                .insert(&transaction)
                .await?
            }
        };
        transaction.commit().await?;
        Ok(row.into())
    }

    pub async fn list_devices(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<Vec<PushDeviceSummary>> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let rows = push_device::Entity::find()
            .filter(push_device::Column::AccountId.eq(actor.account_id()))
            .filter(push_device::Column::ExpiresAt.gt(database_time(&transaction).await?))
            .order_by_asc(push_device::Column::Id)
            .limit(20)
            .all(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn remove_device(
        &self,
        actor: &AuthenticatedSession,
        device_id: Uuid,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let removed = push_device::Entity::delete_many()
            .filter(push_device::Column::Id.eq(device_id))
            .filter(push_device::Column::AccountId.eq(actor.account_id()))
            .exec(&transaction)
            .await?
            .rows_affected;
        if removed != 1 {
            return Err(ApplicationError::NotFound);
        }
        transaction.commit().await?;
        Ok(())
    }
}
