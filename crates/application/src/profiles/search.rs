use super::{PROFILE_SEARCH_JOB, ProfileService, ProfileSummary, entities::profile, repository};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::AuthenticatedSession,
    pagination::{NumberedPageRequest, SEARCH_WINDOW_LIMIT, SearchPage},
    search::versioned_candidates,
};
use jobs::{Claim, HandlerFailure, TraceContext};
use sea_orm::{
    ActiveModelTrait, EntityTrait, IntoActiveModel, Order, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
    sea_query::{LockType, NullOrdering},
};
use uuid::Uuid;

use super::search_jobs::ProfileJob;

impl ProfileService {
    pub async fn initialize_search(&self) -> ApplicationResult<()> {
        if let Some(search) = &self.search {
            search
                .initialize(
                    &["display_name", "bio"],
                    &["organization_ids"],
                    &["id", "version"],
                )
                .await?;
        }
        Ok(())
    }

    pub async fn search_users(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        query: &str,
        page: Option<usize>,
        per_page: Option<usize>,
    ) -> ApplicationResult<SearchPage<ProfileSummary>> {
        let page = NumberedPageRequest::parse(page, per_page)?;
        let search = self
            .search
            .as_ref()
            .ok_or(ApplicationError::IntegrationUnavailable)?;
        self.organizations
            .get_organization(actor, organization_id)
            .await?;
        let hits = search
            .candidates(
                query,
                format!("organization_ids = \"{organization_id}\""),
                SEARCH_WINDOW_LIMIT,
            )
            .await?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let mut rows = repository::visible_in_organization(organization_id)
            .filter(versioned_candidates(
                &hits,
                (profile::Entity, profile::Column::Id),
                (profile::Entity, profile::Column::Version),
            ))
            .all(&transaction)
            .await?;
        rows.sort_by_key(|row| {
            hits.iter()
                .position(|hit| hit.id == row.id)
                .unwrap_or(usize::MAX)
        });
        transaction.commit().await?;
        Ok(page.from_authorized_rows(rows.into_iter().map(Into::into).collect()))
    }

    #[tracing::instrument(skip_all, fields(job_id = %claim.id, request_id = %claim.request_id))]
    pub async fn execute_job(&self, claim: &Claim) -> Result<(), HandlerFailure> {
        if claim.kind != PROFILE_SEARCH_JOB {
            return Err(HandlerFailure {
                code: "unknown_profile_job",
                retryable: false,
            });
        }
        let payload: ProfileJob =
            serde_json::from_value(claim.payload.clone()).map_err(|_| HandlerFailure {
                code: "invalid_profile_payload",
                retryable: false,
            })?;
        self.synchronize_profile(payload.account_id)
            .await
            .map_err(|_| HandlerFailure {
                code: "profile_search_failed",
                retryable: true,
            })
    }

    async fn synchronize_profile(&self, account_id: Uuid) -> ApplicationResult<()> {
        let Some(search) = &self.search else {
            return Ok(());
        };
        let Some(snapshot) = repository::for_account(&self.identity.database, account_id).await?
        else {
            return Ok(());
        };
        let organizations =
            repository::organization_ids(&self.identity.database, account_id).await?;
        let active = repository::active(&self.identity.database, account_id).await?;
        if active && !organizations.is_empty() {
            search.upsert(serde_json::json!({"id":snapshot.id,"version":snapshot.version,"organization_ids":organizations,"display_name":snapshot.display_name,"bio":snapshot.bio})).await?;
        } else {
            search.delete(snapshot.id).await?;
        }
        let transaction = self.identity.database.begin().await?;
        let Some(current) = profile::Entity::find_by_id(snapshot.id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
        else {
            return Ok(());
        };
        if current.version == snapshot.version
            && repository::organization_ids(&transaction, account_id).await? == organizations
            && repository::active(&transaction, account_id).await? == active
        {
            let mut updated = current.into_active_model();
            updated.search_reconciled_at = Set(Some(database_time(&transaction).await?));
            updated.update(&transaction).await?;
        } else {
            super::enqueue_search(&transaction, account_id, TraceContext::default()).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn repair_search(&self) -> ApplicationResult<u64> {
        if self.search.is_none() {
            return Ok(0);
        }
        let transaction = self.identity.database.begin().await?;
        let profiles = profile::Entity::find()
            .order_by_with_nulls(
                profile::Column::SearchReconciledAt,
                Order::Asc,
                NullOrdering::First,
            )
            .order_by_asc(profile::Column::Id)
            .limit(100)
            .all(&transaction)
            .await?;
        for profile in &profiles {
            super::enqueue_search(&transaction, profile.account_id, TraceContext::default())
                .await?;
        }
        transaction.commit().await?;
        Ok(profiles.len() as u64)
    }
}
