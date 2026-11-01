use super::{OrganizationService, repository};
use crate::{ApplicationError, ApplicationResult, identity::AuthenticatedSession};
use sea_orm::TransactionTrait;
use uuid::Uuid;

impl OrganizationService {
    pub async fn add_member(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        account_id: Uuid,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        self.require_owner_in_transaction(&transaction, actor, organization_id, true)
            .await?;
        if repository::is_member(&transaction, organization_id, account_id).await? {
            transaction.commit().await?;
            return Ok(());
        }
        if !repository::account_can_join(&transaction, account_id).await? {
            return Err(ApplicationError::NotFound);
        }
        if repository::count_members(&transaction, organization_id).await? >= 1000 {
            return Err(ApplicationError::Conflict);
        }
        repository::insert_member(&transaction, organization_id, account_id).await?;
        crate::profiles::membership_changed(&transaction, account_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn remove_member(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        account_id: Uuid,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .require_owner_in_transaction(&transaction, actor, organization_id, true)
            .await?;
        if account_id == organization.owner_id {
            return Err(ApplicationError::Forbidden);
        }
        let removed = repository::delete_member(&transaction, organization_id, account_id).await?;
        if removed != 1 {
            return Err(ApplicationError::NotFound);
        }
        crate::profiles::membership_changed(&transaction, account_id).await?;
        transaction.commit().await?;
        Ok(())
    }
}
