use super::{POST_SEARCH_JOB, PostService, PostSummary, entities::post, repository};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::AuthenticatedSession,
    pagination::{NumberedPageRequest, SEARCH_WINDOW_LIMIT, SearchPage},
    posts::PostState,
    search::versioned_candidates,
};
use jobs::{Claim, HandlerFailure, TraceContext};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Order, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
    sea_query::{LockType, NullOrdering},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct PostJob {
    post_id: Uuid,
}

impl PostService {
    pub async fn initialize_search(&self) -> ApplicationResult<()> {
        if let Some(search) = &self.search {
            search
                .initialize(&["title", "body"], &["organization_id"], &["id", "version"])
                .await?;
        }
        Ok(())
    }

    pub async fn search_posts(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        query: &str,
        page: Option<usize>,
        per_page: Option<usize>,
    ) -> ApplicationResult<SearchPage<PostSummary>> {
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
                format!("organization_id = \"{organization_id}\""),
                SEARCH_WINDOW_LIMIT,
            )
            .await?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let mut rows = post::Entity::find()
            .filter(post::Column::OrganizationId.eq(organization_id))
            .filter(repository::published_condition())
            .filter(versioned_candidates(
                &hits,
                post::Column::Id,
                post::Column::Version,
            ))
            .all(&transaction)
            .await?;
        for row in &rows {
            self.require_read(actor, row, organization.owner_id)?;
        }
        rows.sort_by_key(|row| {
            hits.iter()
                .position(|hit| hit.id == row.id)
                .unwrap_or(usize::MAX)
        });
        let page = page.from_authorized_rows(rows);
        let items = self
            .summaries(&transaction, organization_id, page.items)
            .await?;
        transaction.commit().await?;
        Ok(SearchPage {
            items,
            page: page.page,
            per_page: page.per_page,
            total_in_window: page.total_in_window,
            search_window_limit: page.search_window_limit,
            has_next_page: page.has_next_page,
        })
    }

    pub(super) async fn enqueue_search(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        post_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        if self.search.is_some() {
            jobs::enqueue(
                transaction,
                Uuid::now_v7(),
                POST_SEARCH_JOB,
                &PostJob { post_id },
                trace,
            )
            .await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(job_id = %claim.id, request_id = %claim.request_id))]
    pub async fn execute_job(&self, claim: &Claim) -> Result<(), HandlerFailure> {
        if claim.kind != POST_SEARCH_JOB {
            return Err(HandlerFailure {
                code: "unknown_post_job",
                retryable: false,
            });
        }
        let payload: PostJob =
            serde_json::from_value(claim.payload.clone()).map_err(|_| HandlerFailure {
                code: "invalid_post_payload",
                retryable: false,
            })?;
        self.synchronize_post(payload.post_id)
            .await
            .map_err(|_| HandlerFailure {
                code: "post_search_failed",
                retryable: true,
            })
    }

    async fn synchronize_post(&self, post_id: Uuid) -> ApplicationResult<()> {
        let Some(search) = &self.search else {
            return Ok(());
        };
        let Some(snapshot) = post::Entity::find_by_id(post_id)
            .one(&self.identity.database)
            .await?
        else {
            return search.delete(post_id).await;
        };
        if snapshot.state == PostState::Published {
            search.upsert(serde_json::json!({"id":snapshot.id,"organization_id":snapshot.organization_id,"version":snapshot.version,"title":snapshot.title,"body":snapshot.body})).await?;
        } else {
            search.delete(post_id).await?;
        }
        let transaction = self.identity.database.begin().await?;
        let Some(current) = post::Entity::find_by_id(post_id)
            .lock(LockType::NoKeyUpdate)
            .one(&transaction)
            .await?
        else {
            return Ok(());
        };
        if current.version == snapshot.version {
            let mut updated = current.into_active_model();
            updated.search_reconciled_at = Set(Some(database_time(&transaction).await?));
            updated.update(&transaction).await?;
        } else {
            self.enqueue_search(&transaction, post_id, TraceContext::default())
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn repair_search(&self) -> ApplicationResult<u64> {
        if self.search.is_none() {
            return Ok(0);
        }
        let transaction = self.identity.database.begin().await?;
        let rows = post::Entity::find()
            .order_by_with_nulls(
                post::Column::SearchReconciledAt,
                Order::Asc,
                NullOrdering::First,
            )
            .order_by_asc(post::Column::Id)
            .limit(100)
            .all(&transaction)
            .await?;
        for row in &rows {
            self.enqueue_search(&transaction, row.id, TraceContext::default())
                .await?;
        }
        transaction.commit().await?;
        Ok(rows.len() as u64)
    }
}
