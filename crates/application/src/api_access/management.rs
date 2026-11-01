use super::{
    ApiAccessService, ApiClientSummary, ApiKeySummary, IssuedApiKey,
    entities::{api_client, api_key},
    repository,
    types::client_summary,
    validation,
};
use crate::{
    ApplicationError, ApplicationResult,
    api_access::ApiScope,
    config::SecretString,
    database::database_time,
    identity::AuthenticatedSession,
    pagination::{Page, PageRequest},
};
use auth_core::token::{generate_secure_token, hash_token};
use chrono::Duration;
use jobs::TraceContext;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, IntoActiveModel, Set, TransactionTrait};
use uuid::Uuid;

impl ApiAccessService {
    async fn require_owned_client(
        &self,
        transaction: &DatabaseTransaction,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        require_recent_authentication: bool,
    ) -> ApplicationResult<api_client::Model> {
        self.organizations
            .require_owner_in_transaction(
                transaction,
                actor,
                organization_id,
                require_recent_authentication,
            )
            .await?;
        let client = repository::lock_client(transaction, client_id)
            .await?
            .filter(|client| client.organization_id == organization_id)
            .ok_or(ApplicationError::NotFound)?;
        Ok(client)
    }

    #[tracing::instrument(skip_all, fields(organization_id = %organization_id, account_id = %actor.account_id()))]
    pub async fn create_client(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        name: String,
        scopes: Vec<ApiScope>,
        trace: TraceContext,
    ) -> ApplicationResult<ApiClientSummary> {
        let name = validation::name(&name)?;
        validation::scopes(&scopes)?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_owner_in_transaction(&transaction, actor, organization_id, true)
            .await?;
        if repository::client_count(&transaction, organization_id).await?
            >= validation::MAX_CLIENTS_PER_ORGANIZATION
        {
            return Err(ApplicationError::Conflict);
        }
        let client = api_client::ActiveModel {
            id: Set(Uuid::now_v7()),
            organization_id: Set(organization_id),
            name: Set(name),
            scopes: Set(serde_json::json!(scopes)),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "api_client_created",
                "api_client",
                client.id,
                trace,
            )
            .await?;
        let result = client_summary(client, database_time(&transaction).await?)?;
        transaction.commit().await?;
        Ok(result)
    }

    pub async fn list_clients(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<ApiClientSummary>> {
        let scope = format!("api-clients:{organization_id}");
        let page = PageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_owner_in_transaction(&transaction, actor, organization_id, false)
            .await?;
        let now = database_time(&transaction).await?;
        let clients = repository::list_clients(&transaction, organization_id, page)
            .await?
            .into_iter()
            .map(|client| client_summary(client, now))
            .collect::<ApplicationResult<Vec<_>>>()?;
        transaction.commit().await?;
        Ok(Page::from_rows(clients, page, &scope, |client| client.id))
    }

    pub async fn get_client(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
    ) -> ApplicationResult<ApiClientSummary> {
        let transaction = self.identity.database.begin().await?;
        let client = self
            .require_owned_client(&transaction, actor, organization_id, client_id, false)
            .await?;
        let result = client_summary(client, database_time(&transaction).await?)?;
        transaction.commit().await?;
        Ok(result)
    }

    pub async fn update_client(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        name: String,
        scopes: Vec<ApiScope>,
        trace: TraceContext,
    ) -> ApplicationResult<ApiClientSummary> {
        let name = validation::name(&name)?;
        validation::scopes(&scopes)?;
        let transaction = self.identity.database.begin().await?;
        let client = self
            .require_owned_client(&transaction, actor, organization_id, client_id, true)
            .await?;
        let mut updated = client.into_active_model();
        updated.name = Set(name);
        updated.scopes = Set(serde_json::json!(scopes));
        let client = updated.update(&transaction).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "api_client_updated",
                "api_client",
                client.id,
                trace,
            )
            .await?;
        let result = client_summary(client, database_time(&transaction).await?)?;
        transaction.commit().await?;
        Ok(result)
    }

    pub async fn revoke_client(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        let client = self
            .require_owned_client(&transaction, actor, organization_id, client_id, true)
            .await?;
        if client.active {
            let mut client = client.into_active_model();
            client.active = Set(false);
            client.update(&transaction).await?;
            self.identity
                .record_resource_security_event(
                    &transaction,
                    Some(actor.account_id()),
                    "api_client_revoked",
                    "api_client",
                    client_id,
                    trace,
                )
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn issue_key(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        name: String,
        expires_in_days: i64,
        trace: TraceContext,
    ) -> ApplicationResult<IssuedApiKey> {
        let name = validation::name(&name)?;
        validation::key_lifetime(expires_in_days)?;
        let transaction = self.identity.database.begin().await?;
        let client = self
            .require_owned_client(&transaction, actor, organization_id, client_id, true)
            .await?;
        if !client.active {
            return Err(ApplicationError::Conflict);
        }
        if repository::key_count(&transaction, client_id).await? >= validation::MAX_KEYS_PER_CLIENT
        {
            return Err(ApplicationError::Conflict);
        }
        let key_id = Uuid::now_v7();
        let credential = format!("hak_v1_{}.{}", key_id.simple(), generate_secure_token());
        let key = api_key::ActiveModel {
            id: Set(key_id),
            client_id: Set(client_id),
            name: Set(name),
            secret_hash: Set(hash_token(&format!("external API key v1:{credential}"))),
            expires_at: Set(database_time(&transaction).await? + Duration::days(expires_in_days)),
            ..Default::default()
        }
        .insert(&transaction)
        .await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "api_key_issued",
                "api_key",
                key.id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(IssuedApiKey {
            key: key.into(),
            credential: SecretString::new(credential),
        })
    }

    pub async fn list_keys(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<ApiKeySummary>> {
        let scope = format!("api-keys:{client_id}");
        let page = PageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.require_owned_client(&transaction, actor, organization_id, client_id, false)
            .await?;
        let keys = repository::list_keys(&transaction, client_id, page)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        transaction.commit().await?;
        Ok(Page::from_rows(keys, page, &scope, |key| key.id))
    }

    pub async fn revoke_key(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        client_id: Uuid,
        key_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        self.require_owned_client(&transaction, actor, organization_id, client_id, true)
            .await?;
        let key = repository::lock_key(&transaction, key_id)
            .await?
            .filter(|key| key.client_id == client_id)
            .ok_or(ApplicationError::NotFound)?;
        if key.revoked_at.is_none() {
            let mut key = key.into_active_model();
            key.revoked_at = Set(Some(database_time(&transaction).await?));
            key.update(&transaction).await?;
            self.identity
                .record_resource_security_event(
                    &transaction,
                    Some(actor.account_id()),
                    "api_key_revoked",
                    "api_key",
                    key_id,
                    trace,
                )
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(client_id = %client_id, minute_limit, month_limit))]
    pub async fn set_quota_by_operator(
        &self,
        client_id: Uuid,
        minute_limit: i64,
        month_limit: i64,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        validation::quota(minute_limit, month_limit)?;
        let transaction = self.identity.database.begin().await?;
        let client = repository::lock_client(&transaction, client_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        if !client.active {
            return Err(ApplicationError::Conflict);
        }
        let mut client = client.into_active_model();
        client.minute_limit = Set(minute_limit);
        client.month_limit = Set(month_limit);
        client.update(&transaction).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                None,
                "api_client_quota_changed",
                "api_client",
                client_id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
