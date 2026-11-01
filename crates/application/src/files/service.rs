use super::{
    DownloadMethod, FileSettings, FileSummary, PresignedTransfer, ReserveFile, entities::file,
    repository, search::FileSearch, storage::FileStorage,
};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    files::FileState,
    identity::{AuthenticatedSession, IdentityService},
    organizations::OrganizationService,
    pagination::{Page, PageRequest},
};
use chrono::Duration;
use jobs::TraceContext;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub const FILE_DELETE_JOB: &str = "files.delete_original";
pub const FILE_SEARCH_JOB: &str = "files.synchronize_search";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FileJob {
    pub file_id: Uuid,
}

#[derive(Clone)]
pub struct FileService {
    pub(super) identity: IdentityService,
    pub(super) organizations: OrganizationService,
    pub(super) storage: Option<FileStorage>,
    pub(super) search: Option<FileSearch>,
    pub(super) settings: Arc<FileSettings>,
}

impl FileService {
    pub(crate) async fn require_ready_attachments_in_transaction(
        &self,
        transaction: &sea_orm::DatabaseTransaction,
        organization_id: Uuid,
        file_ids: &[Uuid],
    ) -> ApplicationResult<()> {
        let mut file_ids = file_ids.to_vec();
        file_ids.sort();
        for file_id in file_ids {
            let file = repository::lock(transaction, file_id)
                .await?
                .ok_or(ApplicationError::NotFound)?;
            if file.organization_id != organization_id || file.state != FileState::Ready {
                return Err(ApplicationError::NotFound);
            }
        }
        Ok(())
    }
    pub fn new(
        identity: IdentityService,
        organizations: OrganizationService,
        settings: FileSettings,
    ) -> ApplicationResult<Self> {
        settings.validate()?;
        Ok(Self {
            storage: settings.storage.as_ref().map(FileStorage::new),
            search: settings.search.clone().map(FileSearch::new).transpose()?,
            identity,
            organizations,
            settings: Arc::new(settings),
        })
    }

    pub async fn initialize_integrations(&self) -> ApplicationResult<()> {
        if let Some(storage) = &self.storage {
            storage.create_bucket().await?;
            storage
                .configure_browser_transfers(&self.settings.browser_origins)
                .await?;
        }
        if let Some(search) = &self.search {
            search.configure_index().await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), organization_id = %organization_id, request_id = %trace.request_id))]
    pub async fn reserve_file(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        input: ReserveFile,
        trace: TraceContext,
    ) -> ApplicationResult<FileSummary> {
        self.require_storage()?;
        validate_reservation(&input, &self.settings)?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let reserved = repository::reserved_bytes(&transaction, organization_id).await? as u64;
        let now = database_time(&transaction).await?;
        let (live, pending, recent) = repository::reservation_counts(
            &transaction,
            organization_id,
            actor.account_id(),
            now - Duration::minutes(1),
        )
        .await?;
        if recent >= self.settings.reservations_per_minute {
            return Err(ApplicationError::RateLimited);
        }
        if live >= self.settings.organization_file_limit
            || pending >= self.settings.pending_file_limit
        {
            return Err(ApplicationError::FileQuotaExceeded);
        }
        if reserved
            .checked_add(input.byte_size)
            .is_none_or(|bytes| bytes > self.settings.organization_quota_bytes)
        {
            return Err(ApplicationError::FileQuotaExceeded);
        }
        let expires_at =
            now + Duration::seconds(900.max(self.settings.upload_url_seconds as i64 + 120));
        let file = repository::insert_reservation(
            &transaction,
            organization_id,
            actor.account_id(),
            input,
            expires_at,
        )
        .await?;
        transaction.commit().await?;
        Ok(file.into())
    }

    #[tracing::instrument(skip_all, fields(file_id = %file_id, request_id = %trace.request_id))]
    pub async fn create_upload_url(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        file_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<PresignedTransfer> {
        let storage = self.require_storage()?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let file = require_scoped_file(
            repository::lock(&transaction, file_id).await?,
            organization_id,
        )?;
        let now = database_time(&transaction).await?;
        if file.uploader_id != actor.account_id() {
            return Err(ApplicationError::Forbidden);
        }
        if !matches!(file.state, FileState::Uploading | FileState::Transferring)
            || file.upload_expires_at <= now
        {
            return Err(ApplicationError::Conflict);
        }
        let transferring = if file.state == FileState::Uploading {
            repository::set_state(&transaction, file, FileState::Transferring, now, None).await?
        } else {
            file
        };
        transaction.commit().await?;
        storage
            .presign_upload(
                &transferring.object_key,
                &transferring.filename,
                transferring.byte_size as u64,
                now,
                transferring
                    .upload_expires_at
                    .min(now + Duration::seconds(self.settings.upload_url_seconds as i64)),
            )
            .await
    }

    #[tracing::instrument(skip_all, fields(file_id = %file_id, request_id = %trace.request_id))]
    pub async fn complete_upload(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        file_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<FileSummary> {
        let snapshot = self.get_file(actor, organization_id, file_id).await?;
        if snapshot.uploader_id != actor.account_id() {
            return Err(ApplicationError::Forbidden);
        }
        if snapshot.state != FileState::Ready {
            if snapshot.state != FileState::Transferring {
                return Err(ApplicationError::Conflict);
            }
            self.require_storage()?
                .require_original_metadata(
                    &format!("originals/{organization_id}/{file_id}"),
                    snapshot.byte_size,
                )
                .await?;
        }
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let current = require_scoped_file(
            repository::lock(&transaction, file_id).await?,
            organization_id,
        )?;
        let now = database_time(&transaction).await?;
        if current.uploader_id != actor.account_id() {
            return Err(ApplicationError::Forbidden);
        }
        if current.state == FileState::Ready {
            transaction.commit().await?;
            return Ok(current.into());
        }
        if current.state != FileState::Transferring
            || current.version != snapshot.version
            || current.upload_expires_at <= now
        {
            return Err(ApplicationError::Conflict);
        }
        let ready =
            repository::set_state(&transaction, current, FileState::Ready, now, None).await?;
        self.enqueue_search(&transaction, file_id, trace).await?;
        transaction.commit().await?;
        Ok(ready.into())
    }

    pub async fn get_file(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        file_id: Uuid,
    ) -> ApplicationResult<FileSummary> {
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let file = require_scoped_file(
            repository::find(&transaction, file_id).await?,
            organization_id,
        )?;
        if file.state != FileState::Ready && file.uploader_id != actor.account_id() {
            return Err(ApplicationError::NotFound);
        }
        if matches!(file.state, FileState::Deleting | FileState::Deleted) {
            return Err(ApplicationError::NotFound);
        }
        transaction.commit().await?;
        Ok(file.into())
    }

    pub async fn list_files(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<FileSummary>> {
        let scope = format!("files:{organization_id}:{}", actor.account_id());
        let page = PageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let files = repository::list_ready(&transaction, organization_id, page).await?;
        transaction.commit().await?;
        Ok(Page::from_rows(
            files.into_iter().map(Into::into).collect(),
            page,
            &scope,
            |file: &FileSummary| file.id,
        ))
    }

    pub async fn create_download_url(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        file_id: Uuid,
        method: DownloadMethod,
    ) -> ApplicationResult<PresignedTransfer> {
        let storage = self.require_storage()?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let file = require_scoped_file(
            repository::find(&transaction, file_id).await?,
            organization_id,
        )?;
        if file.state != FileState::Ready {
            return Err(ApplicationError::NotFound);
        }
        let now = database_time(&transaction).await?;
        transaction.commit().await?;
        storage
            .presign_download(
                &file.object_key,
                &file.filename,
                method,
                now,
                now + Duration::seconds(self.settings.download_url_seconds as i64),
            )
            .await
    }

    #[tracing::instrument(skip_all, fields(file_id = %file_id, request_id = %trace.request_id))]
    pub async fn delete_file(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        file_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        let organization = self
            .organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let file = require_scoped_file(
            repository::lock(&transaction, file_id).await?,
            organization_id,
        )?;
        if file.uploader_id != actor.account_id() && organization.owner_id != actor.account_id() {
            return Err(ApplicationError::Forbidden);
        }
        if !matches!(file.state, FileState::Deleting | FileState::Deleted) {
            let now = database_time(&transaction).await?;
            repository::set_state(&transaction, file, FileState::Deleting, now, None).await?;
            jobs::enqueue(
                &transaction,
                Uuid::now_v7(),
                FILE_DELETE_JOB,
                &FileJob { file_id },
                trace,
            )
            .await?;
            self.enqueue_search(&transaction, file_id, trace).await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn search_files(
        &self,
        actor: &AuthenticatedSession,
        organization_id: Uuid,
        query: &str,
    ) -> ApplicationResult<Vec<FileSummary>> {
        let search = self
            .search
            .as_ref()
            .ok_or(ApplicationError::IntegrationUnavailable)?;
        self.organizations
            .get_organization(actor, organization_id)
            .await?;
        let candidates = search.candidates(organization_id, query).await?;
        let transaction = self.identity.database.begin().await?;
        self.organizations
            .require_member_in_transaction(&transaction, actor, organization_id)
            .await?;
        let mut files =
            repository::ready_candidates(&transaction, organization_id, candidates.clone()).await?;
        transaction.commit().await?;
        files.sort_by_key(|file| {
            candidates
                .iter()
                .position(|id| *id == file.id)
                .unwrap_or(usize::MAX)
        });
        Ok(files.into_iter().take(20).map(Into::into).collect())
    }

    pub(super) async fn enqueue_search(
        &self,
        transaction: &DatabaseTransaction,
        file_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        if self.search.is_some() {
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
            file::Entity::update_many()
                .col_expr(
                    file::Column::SearchDocumentMayExist,
                    sea_orm::sea_query::Expr::value(true),
                )
                .filter(file::Column::Id.eq(file_id))
                .filter(file::Column::State.eq(FileState::Ready))
                .exec(transaction)
                .await?;
            jobs::enqueue(
                transaction,
                Uuid::now_v7(),
                FILE_SEARCH_JOB,
                &FileJob { file_id },
                trace,
            )
            .await?;
        }
        Ok(())
    }

    pub(super) fn require_storage(&self) -> ApplicationResult<&FileStorage> {
        self.storage
            .as_ref()
            .ok_or(ApplicationError::IntegrationUnavailable)
    }
}

fn require_scoped_file(
    file: Option<file::Model>,
    organization_id: Uuid,
) -> ApplicationResult<file::Model> {
    file.filter(|file| file.organization_id == organization_id)
        .ok_or(ApplicationError::NotFound)
}

fn validate_reservation(input: &ReserveFile, settings: &FileSettings) -> ApplicationResult<()> {
    if input.byte_size > settings.max_file_bytes {
        return Err(ApplicationError::FileTooLarge);
    }
    super::validation::validate_filename(&input.filename)?;
    if let Some(media_type) = &input.media_type {
        super::validation::validate_media_type(media_type)?;
    }
    Ok(())
}
