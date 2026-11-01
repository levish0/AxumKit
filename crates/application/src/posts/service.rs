use super::{
    PostPublication, PostSummary, SavePost,
    entities::{post, post_attachment},
    repository,
    types::facts,
    validation,
};
use crate::{
    ApplicationError, ApplicationResult,
    authorization::Action,
    database::database_time,
    files::FileService,
    identity::{AuthenticatedSession, IdentityService},
    notifications::NotificationService,
    organizations::OrganizationService,
    pagination::{ChronologicalPageRequest, ChronologicalPosition, Page},
    posts::PostState,
    search::{SearchIndex, SearchSettings},
};
use jobs::TraceContext;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

pub const POST_SEARCH_JOB: &str = "posts.search";

#[derive(Clone)]
pub struct PostService {
    pub(super) identity: IdentityService,
    pub(super) organizations: OrganizationService,
    pub(super) search: Option<SearchIndex>,
    files: FileService,
    pub(super) notifications: NotificationService,
}

impl PostService {
    pub fn new(
        identity: IdentityService,
        organizations: OrganizationService,
        files: FileService,
        notifications: NotificationService,
        search: Option<SearchSettings>,
    ) -> ApplicationResult<Self> {
        Ok(Self {
            identity,
            organizations,
            files,
            notifications,
            search: search.map(SearchIndex::new).transpose()?,
        })
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), organization_id = %organization_id, request_id = %trace.request_id))]
    pub async fn create_post(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        input: SavePost,
        trace: TraceContext,
    ) -> ApplicationResult<PostSummary> {
        self.validate_input(&input)?;
        if input.expected_version != 0 {
            return Err(ApplicationError::InvalidInput(
                "new posts require version zero",
            ));
        }
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let now = database_time(&transaction).await?;
        let row = post::Model {
            id: Uuid::now_v7(),
            organization_id,
            author_id: actor.account_id(),
            title: input.title.trim().into(),
            body: input.body,
            state: input.state.into(),
            version: 1,
            created_at: now,
            updated_at: now,
            search_reconciled_at: None,
        };
        self.identity.authorization.require_post(
            actor.principal,
            Action::CreatePost,
            facts(&row, organization.owner_id),
        )?;
        let row = row.into_active_model().insert(&transaction).await?;
        self.replace_attachments(&transaction, &row, &input.attachment_ids)
            .await?;
        self.enqueue_search(&transaction, row.id, trace).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "post_created",
                "post",
                row.id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(PostSummary::from_model(row, input.attachment_ids))
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), organization_id = %organization_id, post_id = %post_id, request_id = %trace.request_id))]
    pub async fn update_post(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
        input: SavePost,
        trace: TraceContext,
    ) -> ApplicationResult<PostSummary> {
        self.validate_input(&input)?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let current = repository::lock(&transaction, organization_id, post_id)
            .await?
            .filter(|row| row.state != PostState::Deleted)
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &current, organization.owner_id)?;
        self.identity.authorization.require_post(
            actor.principal,
            Action::EditPost,
            facts(&current, organization.owner_id),
        )?;
        if current.version != input.expected_version {
            return Err(ApplicationError::Conflict);
        }
        let version = current
            .version
            .checked_add(1)
            .ok_or(ApplicationError::Conflict)?;
        let mut updated = current.into_active_model();
        updated.title = Set(input.title.trim().into());
        updated.body = Set(input.body);
        updated.state = Set(input.state.into());
        updated.version = Set(version);
        updated.updated_at = Set(database_time(&transaction).await?);
        let row = updated.update(&transaction).await?;
        self.replace_attachments(&transaction, &row, &input.attachment_ids)
            .await?;
        self.enqueue_search(&transaction, row.id, trace).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "post_updated",
                "post",
                row.id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(PostSummary::from_model(row, input.attachment_ids))
    }

    pub async fn get_post(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
    ) -> ApplicationResult<PostSummary> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let row = repository::lock(&transaction, organization_id, post_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &row, organization.owner_id)?;
        let mut attachments =
            repository::attachments(&transaction, organization_id, vec![post_id]).await?;
        transaction.commit().await?;
        Ok(PostSummary::from_model(
            row,
            attachments.remove(&post_id).unwrap_or_default(),
        ))
    }

    pub async fn list_posts(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        state: PostPublication,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<PostSummary>> {
        let scope = format!(
            "posts:{organization_id}:{}:{}",
            actor.account_id(),
            state.as_str()
        );
        let page = ChronologicalPageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let mut query = post::Entity::find()
            .filter(post::Column::OrganizationId.eq(organization_id))
            .filter(post::Column::State.eq(PostState::from(state)));
        if state == PostPublication::Draft && organization.owner_id != actor.account_id() {
            query = query.filter(post::Column::AuthorId.eq(actor.account_id()));
        }
        let rows = query
            .filter(page.condition(post::Column::CreatedAt, post::Column::Id))
            .order_by_desc(post::Column::CreatedAt)
            .order_by_desc(post::Column::Id)
            .limit(page.limit + 1)
            .all(&transaction)
            .await?;
        for row in &rows {
            self.require_read(actor, row, organization.owner_id)?;
        }
        let page = Page::from_chronological_rows(rows, page, &scope, |row| ChronologicalPosition {
            created_at: row.created_at,
            id: row.id,
        });
        let items = self
            .summaries(&transaction, organization_id, page.items)
            .await?;
        transaction.commit().await?;
        Ok(Page {
            items,
            next_cursor: page.next_cursor,
        })
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), post_id = %post_id, request_id = %trace.request_id))]
    pub async fn delete_post(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        post_id: Uuid,
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
            .filter(|row| row.state != PostState::Deleted)
            .ok_or(ApplicationError::NotFound)?;
        self.require_read(actor, &row, organization.owner_id)?;
        self.identity.authorization.require_post(
            actor.principal,
            Action::DeletePost,
            facts(&row, organization.owner_id),
        )?;
        if row.version != expected_version {
            return Err(ApplicationError::Conflict);
        }
        let version = row
            .version
            .checked_add(1)
            .ok_or(ApplicationError::Conflict)?;
        let mut updated = row.into_active_model();
        updated.state = Set(PostState::Deleted);
        updated.title = Set(String::new());
        updated.body = Set(String::new());
        updated.version = Set(version);
        updated.updated_at = Set(database_time(&transaction).await?);
        updated.update(&transaction).await?;
        self.enqueue_search(&transaction, post_id, trace).await?;
        self.identity
            .record_resource_security_event(
                &transaction,
                Some(actor.account_id()),
                "post_deleted",
                "post",
                post_id,
                trace,
            )
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    fn validate_input(&self, input: &SavePost) -> ApplicationResult<()> {
        validation::title(&input.title)?;
        validation::body(&input.body)?;
        validation::attachments(&input.attachment_ids)?;
        if input.expected_version < 0 {
            return Err(ApplicationError::InvalidInput("invalid post version"));
        }
        Ok(())
    }

    pub(super) fn require_read(
        &self,
        actor: &AuthenticatedSession,
        row: &post::Model,
        owner_id: Uuid,
    ) -> ApplicationResult<()> {
        match self.identity.authorization.require_post(
            actor.principal,
            Action::ReadPost,
            facts(row, owner_id),
        ) {
            Err(ApplicationError::Forbidden) => Err(ApplicationError::NotFound),
            other => other,
        }
    }

    async fn replace_attachments(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        row: &post::Model,
        ids: &[Uuid],
    ) -> ApplicationResult<()> {
        self.files
            .require_ready_attachments_in_transaction(transaction, row.organization_id, ids)
            .await?;
        post_attachment::Entity::delete_many()
            .filter(post_attachment::Column::PostId.eq(row.id))
            .exec(transaction)
            .await?;
        for file_id in ids {
            post_attachment::ActiveModel {
                id: Set(Uuid::now_v7()),
                post_id: Set(row.id),
                file_id: Set(*file_id),
            }
            .insert(transaction)
            .await?;
        }
        Ok(())
    }

    pub(super) async fn summaries(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        organization_id: Uuid,
        rows: Vec<post::Model>,
    ) -> ApplicationResult<Vec<PostSummary>> {
        let mut attachments = repository::attachments(
            transaction,
            organization_id,
            rows.iter().map(|row| row.id).collect(),
        )
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let ids = attachments.remove(&row.id).unwrap_or_default();
                PostSummary::from_model(row, ids)
            })
            .collect())
    }
}
