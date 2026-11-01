use super::{entities::organization, repository, validation::normalize_name};
use crate::{
    ApplicationError, ApplicationResult,
    authorization::Action,
    identity::{AuthenticatedSession, IdentityService},
    pagination::{Page, PageRequest},
};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use uuid::Uuid;

#[derive(Clone)]
pub struct OrganizationService {
    pub(super) identity: IdentityService,
}

pub struct OrganizationSummary {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub version: i64,
}
impl From<organization::Model> for OrganizationSummary {
    fn from(organization: organization::Model) -> Self {
        Self {
            id: organization.id,
            name: organization.name,
            owner_id: organization.owner_id,
            version: organization.version,
        }
    }
}

impl OrganizationService {
    pub fn new(identity: IdentityService) -> Self {
        Self { identity }
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id()))]
    pub async fn create_organization(
        &self,
        actor: &AuthenticatedSession,
        name: String,
    ) -> ApplicationResult<OrganizationSummary> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .create_organization_in_transaction(&transaction, actor, name)
            .await?;
        transaction.commit().await?;
        Ok(organization)
    }

    pub async fn create_organization_in_transaction(
        &self,
        transaction: &DatabaseTransaction,
        actor: &AuthenticatedSession,
        name: String,
    ) -> ApplicationResult<OrganizationSummary> {
        let name = normalize_name(&name)?;
        self.identity
            .lock_authenticated_account(transaction, actor, false)
            .await?;
        let organization =
            repository::insert_organization(transaction, name, actor.account_id()).await?;
        crate::profiles::membership_changed(transaction, actor.account_id()).await?;
        Ok(organization.into())
    }

    pub async fn get_organization(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
    ) -> ApplicationResult<OrganizationSummary> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        transaction.commit().await?;
        Ok(organization)
    }

    pub async fn require_member_in_transaction(
        &self,
        transaction: &DatabaseTransaction,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
    ) -> ApplicationResult<OrganizationSummary> {
        let account = self
            .identity
            .lock_authenticated_account(transaction, actor, false)
            .await?;
        let organization = repository::lock_organization(transaction, organization_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        let member = repository::is_member(transaction, organization.id, account.id).await?;
        if !member {
            return Err(ApplicationError::NotFound);
        }
        self.identity.authorization.require(
            crate::identity::service::principal(&account),
            Action::ReadOrganization,
            organization.id,
            organization.owner_id,
            member,
        )?;
        Ok(organization.into())
    }

    pub async fn list_organizations(
        &self,
        actor: &AuthenticatedSession,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<OrganizationSummary>> {
        let scope = format!("organizations:{}", actor.account_id());
        let page = PageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let organizations =
            repository::list_for_member(&transaction, actor.account_id(), page).await?;
        transaction.commit().await?;
        Ok(Page::from_rows(
            organizations
                .into_iter()
                .map(OrganizationSummary::from)
                .collect(),
            page,
            &scope,
            |organization| organization.id,
        ))
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), organization_id = %organization_id))]
    pub async fn rename_organization(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        expected_version: i64,
        name: String,
    ) -> ApplicationResult<OrganizationSummary> {
        let name = normalize_name(&name)?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .require_owner_in_transaction(&transaction, actor, organization_id, false)
            .await?;
        if organization.version != expected_version {
            return Err(ApplicationError::Conflict);
        }
        let updated = repository::rename_organization(&transaction, organization, name).await?;
        transaction.commit().await?;
        Ok(updated.into())
    }

    pub(crate) async fn require_owner_in_transaction(
        &self,
        transaction: &DatabaseTransaction,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        require_recent_authentication: bool,
    ) -> ApplicationResult<organization::Model> {
        let account = self
            .identity
            .lock_authenticated_account(transaction, actor, require_recent_authentication)
            .await?;
        let organization = repository::lock_organization(transaction, organization_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        let member = repository::is_member(transaction, organization.id, account.id).await?;
        if !member {
            return Err(ApplicationError::NotFound);
        }
        self.identity.authorization.require(
            crate::identity::service::principal(&account),
            Action::ManageOrganization,
            organization.id,
            organization.owner_id,
            member,
        )?;
        Ok(organization)
    }

    pub(crate) async fn read_for_api_client(
        &self,
        transaction: &DatabaseTransaction,
        organization_id: Uuid,
    ) -> ApplicationResult<OrganizationSummary> {
        repository::find_organization(transaction, organization_id)
            .await?
            .map(Into::into)
            .ok_or(ApplicationError::NotFound)
    }
}
