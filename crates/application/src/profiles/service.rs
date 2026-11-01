use super::{ProfileSummary, SaveProfile, entities::profile, repository, validation};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::{AuthenticatedSession, IdentityService},
    organizations::OrganizationService,
    pagination::{ChronologicalPageRequest, ChronologicalPosition, Page},
    search::{SearchIndex, SearchSettings},
};
use jobs::TraceContext;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;

pub const PROFILE_SEARCH_JOB: &str = "profiles.search";

#[derive(Clone)]
pub struct ProfileService {
    pub(super) identity: IdentityService,
    pub(super) organizations: OrganizationService,
    pub(super) search: Option<SearchIndex>,
}

impl ProfileService {
    pub fn new(
        identity: IdentityService,
        organizations: OrganizationService,
        settings: Option<SearchSettings>,
    ) -> ApplicationResult<Self> {
        Ok(Self {
            identity,
            organizations,
            search: settings.map(SearchIndex::new).transpose()?,
        })
    }

    pub async fn get_own_profile(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<ProfileSummary> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let profile = repository::for_account(&transaction, actor.account_id())
            .await?
            .ok_or(ApplicationError::NotFound)?;
        transaction.commit().await?;
        Ok(profile.into())
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), request_id = %trace.request_id))]
    pub async fn save_profile(
        &self,
        actor: &AuthenticatedSession,
        input: SaveProfile,
        trace: TraceContext,
    ) -> ApplicationResult<ProfileSummary> {
        validation::display_name(&input.display_name)?;
        validation::bio(&input.bio)?;
        if input.expected_version < 0 {
            return Err(ApplicationError::InvalidInput("invalid profile version"));
        }
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let now = database_time(&transaction).await?;
        let profile = match repository::for_account(&transaction, actor.account_id()).await? {
            Some(current) if current.version == input.expected_version => {
                let version = current
                    .version
                    .checked_add(1)
                    .ok_or(ApplicationError::Conflict)?;
                let mut updated = current.into_active_model();
                updated.display_name = Set(input.display_name.trim().into());
                updated.bio = Set(input.bio);
                updated.version = Set(version);
                updated.updated_at = Set(now);
                updated.update(&transaction).await?
            }
            None if input.expected_version == 0 => {
                profile::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    account_id: Set(actor.account_id()),
                    display_name: Set(input.display_name.trim().into()),
                    bio: Set(input.bio),
                    version: Set(1),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
                .insert(&transaction)
                .await?
            }
            _ => return Err(ApplicationError::Conflict),
        };
        if self.search.is_some() {
            super::enqueue_search(&transaction, actor.account_id(), trace).await?;
        }
        transaction.commit().await?;
        Ok(profile.into())
    }

    pub async fn get_user(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        account_id: Uuid,
    ) -> ApplicationResult<ProfileSummary> {
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let profile = repository::visible_in_organization(organization_id)
            .filter(profile::Column::AccountId.eq(account_id))
            .one(&transaction)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        transaction.commit().await?;
        Ok(profile.into())
    }

    pub async fn list_users(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<ProfileSummary>> {
        let scope = format!("users:{organization_id}:{}", actor.account_id());
        let page = ChronologicalPageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let rows = repository::visible_in_organization(organization_id)
            .filter(page.condition(
                (profile::Entity, profile::Column::CreatedAt),
                (profile::Entity, profile::Column::Id),
            ))
            .order_by_desc(profile::Column::CreatedAt)
            .order_by_desc(profile::Column::Id)
            .limit(page.limit + 1)
            .all(&transaction)
            .await?;
        let result =
            Page::from_chronological_rows(rows, page, &scope, |row| ChronologicalPosition {
                created_at: row.created_at,
                id: row.id,
            });
        transaction.commit().await?;
        Ok(Page {
            items: result.items.into_iter().map(Into::into).collect(),
            next_cursor: result.next_cursor,
        })
    }
}
