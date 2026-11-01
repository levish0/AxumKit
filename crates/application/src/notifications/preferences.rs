use super::{
    NotificationPreferences, NotificationService, entities::notification_preference, repository,
};
use crate::{ApplicationResult, identity::AuthenticatedSession};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait,
};
use uuid::Uuid;

impl NotificationService {
    pub async fn get_preferences(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<NotificationPreferences> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let result = repository::preferences(&transaction, actor.account_id()).await?;
        transaction.commit().await?;
        Ok(result)
    }

    pub async fn save_preferences(
        &self,
        actor: &AuthenticatedSession,
        preferences: NotificationPreferences,
    ) -> ApplicationResult<NotificationPreferences> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let current = notification_preference::Entity::find()
            .filter(notification_preference::Column::AccountId.eq(actor.account_id()))
            .one(&transaction)
            .await?;
        match current {
            Some(current) => {
                let mut updated = current.into_active_model();
                updated.post_comments = Set(preferences.post_comments);
                updated.push_enabled = Set(preferences.push_enabled);
                updated.update(&transaction).await?;
            }
            None => {
                notification_preference::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    account_id: Set(actor.account_id()),
                    post_comments: Set(preferences.post_comments),
                    push_enabled: Set(preferences.push_enabled),
                }
                .insert(&transaction)
                .await?;
            }
        }
        transaction.commit().await?;
        Ok(preferences)
    }
}
