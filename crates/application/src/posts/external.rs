use super::{PostService, PostSummary, entities::post, repository, types::facts};
use crate::{ApplicationError, ApplicationResult, api_access::ApiScope};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait};

impl PostService {
    pub async fn list_for_api_client(
        &self,
        grant: &crate::api_access::ApiAdmission,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<crate::pagination::Page<PostSummary>> {
        if grant.scope() != ApiScope::PostsRead {
            return Err(ApplicationError::Forbidden);
        }
        let scope = format!(
            "external_posts:{}:{}",
            grant.organization_id(),
            grant.client_id()
        );
        let page = crate::pagination::ChronologicalPageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        let rows = post::Entity::find()
            .filter(post::Column::OrganizationId.eq(grant.organization_id()))
            .filter(repository::published_condition())
            .filter(page.condition(post::Column::CreatedAt, post::Column::Id))
            .order_by_desc(post::Column::CreatedAt)
            .order_by_desc(post::Column::Id)
            .limit(page.limit + 1)
            .all(&transaction)
            .await?;
        for row in &rows {
            self.identity.authorization.require_external_post(
                grant.client_id(),
                grant.organization_id(),
                facts(row, grant.organization_owner_id()),
            )?;
        }
        let page = crate::pagination::Page::from_chronological_rows(rows, page, &scope, |row| {
            crate::pagination::ChronologicalPosition {
                created_at: row.created_at,
                id: row.id,
            }
        });
        let items = self
            .summaries(&transaction, grant.organization_id(), page.items)
            .await?;
        transaction.commit().await?;
        Ok(crate::pagination::Page {
            items,
            next_cursor: page.next_cursor,
        })
    }
}
