use super::{
    FILE_DELETE_JOB, FILE_SEARCH_JOB, FileService, entities::file, repository, service::FileJob,
};
use crate::{ApplicationError, ApplicationResult, database::database_time, files::FileState};
use jobs::{Claim, HandlerFailure, TraceContext};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, Order, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    TransactionTrait,
    sea_query::{Expr, NullOrdering},
};
use uuid::Uuid;

impl FileService {
    #[tracing::instrument(skip_all, fields(job_id = %claim.id, request_id = %claim.request_id))]
    pub async fn execute_job(&self, claim: &Claim) -> Result<(), HandlerFailure> {
        let payload: FileJob =
            serde_json::from_value(claim.payload.clone()).map_err(|_| HandlerFailure {
                code: "invalid_file_payload",
                retryable: false,
            })?;
        let result = match claim.kind.as_str() {
            FILE_DELETE_JOB => self.remove_original(payload.file_id).await,
            FILE_SEARCH_JOB => self.synchronize_search(payload.file_id).await,
            _ => {
                return Err(HandlerFailure {
                    code: "unknown_job_kind",
                    retryable: false,
                });
            }
        };
        result.map_err(|_| HandlerFailure {
            code: "file_integration_failed",
            retryable: true,
        })
    }

    async fn remove_original(&self, file_id: Uuid) -> ApplicationResult<()> {
        let Some(snapshot) = repository::find(&self.identity.database, file_id).await? else {
            return Ok(());
        };
        if !matches!(snapshot.state, FileState::Deleting | FileState::Deleted) {
            return Ok(());
        }
        let deletion_started_at = database_time(&self.identity.database).await?;
        self.require_storage()?
            .delete_original(&snapshot.object_key)
            .await?;
        let transaction = self.identity.database.begin().await?;
        let Some(current) = repository::lock(&transaction, file_id).await? else {
            return Ok(());
        };
        let now = database_time(&transaction).await?;
        // A signed PUT can still finish after expiry. Keep its quota until ingress's deadline.
        if current.state == FileState::Deleting
            && current.upload_expires_at
                + chrono::Duration::seconds(self.settings.upload_grace_seconds as i64)
                < deletion_started_at
        {
            repository::set_state(&transaction, current, FileState::Deleted, now, None).await?;
            self.enqueue_search(&transaction, file_id, TraceContext::default())
                .await?;
        }
        let now = database_time(&transaction).await?;
        file::Entity::update_many()
            .col_expr(file::Column::CleanupCompletedAt, Expr::value(Some(now)))
            .filter(file::Column::Id.eq(file_id))
            .filter(file::Column::State.is_in([FileState::Deleting, FileState::Deleted]))
            .exec(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn synchronize_search(&self, file_id: Uuid) -> ApplicationResult<()> {
        let search = self
            .search
            .as_ref()
            .ok_or(ApplicationError::IntegrationUnavailable)?;
        for _ in 0..3 {
            let Some(snapshot) = repository::find(&self.identity.database, file_id).await? else {
                return Ok(());
            };
            search.synchronize(&snapshot).await?;
            let transaction = self.identity.database.begin().await?;
            let now = database_time(&transaction).await?;
            let current = repository::record_search_reconciliation(
                &transaction,
                file_id,
                snapshot.version,
                now,
            )
            .await?;
            transaction.commit().await?;
            if current {
                return Ok(());
            }
        }
        Err(ApplicationError::IntegrationUnavailable)
    }

    pub async fn reconcile_files(&self) -> ApplicationResult<u64> {
        if self.storage.is_none() {
            return Ok(0);
        }
        let transaction = self.identity.database.begin().await?;
        let now = database_time(&transaction).await?;
        let purge = file::Entity::find()
            .select_only()
            .column(file::Column::Id)
            .filter(file::Column::State.eq(FileState::Deleted))
            .filter(file::Column::SearchDocumentMayExist.eq(false))
            .filter(
                file::Column::UpdatedAt
                    .lt(now - chrono::Duration::days(self.settings.tombstone_days)),
            )
            .filter(
                file::Column::UploadExpiresAt
                    .lt(now - chrono::Duration::seconds(self.settings.upload_grace_seconds as i64)),
            )
            .filter(file::Column::CleanupCompletedAt.gte(now - chrono::Duration::days(1)));
        let purge = purge.limit(100).into_query();
        file::Entity::delete_many()
            .filter(file::Column::Id.in_subquery(purge))
            .filter(file::Column::State.eq(FileState::Deleted))
            .exec(&transaction)
            .await?;
        let candidates = file::Entity::find()
            .filter(
                Condition::any()
                    .add(
                        Condition::all()
                            .add(
                                file::Column::State
                                    .is_in([FileState::Uploading, FileState::Transferring]),
                            )
                            .add(file::Column::UploadExpiresAt.lte(now)),
                    )
                    .add(
                        Condition::all()
                            .add(
                                file::Column::State
                                    .is_in([FileState::Deleting, FileState::Deleted]),
                            )
                            .add(
                                Condition::any()
                                    .add(file::Column::CleanupScheduledAt.is_null())
                                    .add(
                                        file::Column::CleanupScheduledAt
                                            .lt(now - chrono::Duration::hours(1)),
                                    ),
                            ),
                    ),
            )
            .order_by_with_nulls(
                file::Column::CleanupScheduledAt,
                Order::Asc,
                NullOrdering::First,
            )
            .order_by_asc(file::Column::Id)
            .limit(100)
            .lock_with_behavior(
                sea_orm::sea_query::LockType::Update,
                sea_orm::sea_query::LockBehavior::SkipLocked,
            )
            .all(&transaction)
            .await?;
        let count = candidates.len() as u64;
        for candidate in candidates {
            let file_id = candidate.id;
            if matches!(
                candidate.state,
                FileState::Uploading | FileState::Transferring
            ) {
                repository::set_state(&transaction, candidate, FileState::Deleting, now, None)
                    .await?;
                self.enqueue_search(&transaction, file_id, TraceContext::default())
                    .await?;
            }
            file::Entity::update_many()
                .col_expr(file::Column::CleanupScheduledAt, Expr::value(Some(now)))
                .filter(file::Column::Id.eq(file_id))
                .exec(&transaction)
                .await?;
            jobs::enqueue(
                &transaction,
                Uuid::now_v7(),
                FILE_DELETE_JOB,
                &FileJob { file_id },
                TraceContext::default(),
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(count)
    }

    pub async fn repair_search(&self) -> ApplicationResult<u64> {
        if self.search.is_none() {
            return Ok(0);
        }
        let transaction = self.identity.database.begin().await?;
        let candidates = file::Entity::find()
            .order_by_with_nulls(
                file::Column::SearchReconciledAt,
                Order::Asc,
                NullOrdering::First,
            )
            .order_by_asc(file::Column::Id)
            .limit(100)
            .all(&transaction)
            .await?;
        for candidate in &candidates {
            self.enqueue_search(&transaction, candidate.id, TraceContext::default())
                .await?;
        }
        transaction.commit().await?;
        Ok(candidates.len() as u64)
    }
}
