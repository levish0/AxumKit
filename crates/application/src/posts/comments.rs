use super::{CommentSummary, PostService, repository, types::facts, validation};
use crate::{
    ApplicationError, ApplicationResult,
    authorization::Action,
    database::database_time,
    identity::AuthenticatedSession,
    pagination::{ChronologicalPageRequest, ChronologicalPosition, Page},
    posts::PostState,
};
use jobs::TraceContext;
use sea_orm::TransactionTrait;
use uuid::Uuid;

impl PostService {
    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), post_id = %post_id, request_id = %trace.request_id))]
    pub async fn create_comment(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
        body: String,
        trace: TraceContext,
    ) -> ApplicationResult<CommentSummary> {
        validation::comment(&body)?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let row = repository::lock(&transaction, organization_id, post_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &row, organization.owner_id)?;
        self.identity.authorization.require_post(
            actor.principal,
            Action::CreateComment,
            facts(&row, organization.owner_id),
        )?;
        let comment = repository::insert_comment(
            &transaction,
            post_id,
            actor.account_id(),
            body,
            database_time(&transaction).await?,
        )
        .await?;
        self.notifications
            .notify_comment_in_transaction(&transaction, &row, &comment, trace)
            .await?;
        transaction.commit().await?;
        Ok(comment.into())
    }

    pub async fn list_comments(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<CommentSummary>> {
        let scope = format!(
            "comments:{organization_id}:{post_id}:{}",
            actor.account_id()
        );
        let page = ChronologicalPageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let row = repository::lock(&transaction, organization_id, post_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &row, organization.owner_id)?;
        if row.state != PostState::Published {
            return Err(ApplicationError::NotFound);
        }
        let rows = repository::list_comments(&transaction, post_id, &page).await?;
        let page = Page::from_chronological_rows(rows, page, &scope, |row| ChronologicalPosition {
            created_at: row.created_at,
            id: row.id,
        });
        transaction.commit().await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        })
    }

    pub async fn delete_comment(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
        comment_id: Uuid,
        expected_version: i64,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let row = repository::lock(&transaction, organization_id, post_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &row, organization.owner_id)?;
        let comment = repository::lock_comment(&transaction, post_id, comment_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        self.identity.authorization.require_delete_comment(
            actor.principal,
            comment.id,
            comment.author_id,
            comment.deleted,
            facts(&row, organization.owner_id),
        )?;
        if comment.version != expected_version {
            return Err(ApplicationError::Conflict);
        }
        let version = comment
            .version
            .checked_add(1)
            .ok_or(ApplicationError::Conflict)?;
        repository::tombstone_comment(&transaction, comment, version).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "comment_deleted",
                "comment",
                comment_id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
