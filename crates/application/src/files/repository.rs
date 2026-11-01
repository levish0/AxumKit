use super::{ReserveFile, entities::file};
use crate::{files::FileState, pagination::PageRequest};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    ExprTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::Expr,
};
use uuid::Uuid;

pub async fn insert_reservation(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    uploader_id: Uuid,
    input: ReserveFile,
    expires_at: DateTime<Utc>,
) -> Result<file::Model, DbErr> {
    let id = Uuid::now_v7();
    file::ActiveModel {
        id: Set(id),
        organization_id: Set(organization_id),
        uploader_id: Set(uploader_id),
        object_key: Set(format!("originals/{organization_id}/{id}")),
        filename: Set(input.filename),
        media_type: Set(input.media_type),
        byte_size: Set(input.byte_size as i64),
        state: Set(FileState::Uploading),
        upload_expires_at: Set(expires_at),
        ..Default::default()
    }
    .insert(transaction)
    .await
}

pub async fn reserved_bytes(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<i64, DbErr> {
    let bytes: Option<i64> = file::Entity::find()
        .select_only()
        .expr_as(
            Expr::col(file::Column::ByteSize)
                .sum()
                .cast_as(sea_orm::sea_query::Alias::new("bigint")),
            "reserved_bytes",
        )
        .filter(file::Column::OrganizationId.eq(organization_id))
        .filter(file::Column::State.ne(FileState::Deleted))
        .into_tuple()
        .one(transaction)
        .await?
        .flatten();
    Ok(bytes.unwrap_or(0))
}

pub async fn find(
    connection: &impl ConnectionTrait,
    file_id: Uuid,
) -> Result<Option<file::Model>, DbErr> {
    file::Entity::find_by_id(file_id).one(connection).await
}

pub async fn reservation_counts(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    uploader_id: Uuid,
    since: DateTime<Utc>,
) -> Result<(u64, u64, u64), DbErr> {
    let organization_files =
        file::Entity::find().filter(file::Column::OrganizationId.eq(organization_id));
    let live = organization_files
        .clone()
        .filter(file::Column::State.ne(FileState::Deleted))
        .count(transaction)
        .await?;
    let pending = organization_files
        .filter(file::Column::State.is_in([FileState::Uploading, FileState::Transferring]))
        .count(transaction)
        .await?;
    let recent = file::Entity::find()
        .filter(file::Column::UploaderId.eq(uploader_id))
        .filter(file::Column::CreatedAt.gte(since))
        .count(transaction)
        .await?;
    Ok((live, pending, recent))
}

pub async fn lock(
    transaction: &DatabaseTransaction,
    file_id: Uuid,
) -> Result<Option<file::Model>, DbErr> {
    file::Entity::find_by_id(file_id)
        .lock_exclusive()
        .one(transaction)
        .await
}

pub async fn set_state(
    transaction: &DatabaseTransaction,
    file: file::Model,
    state: FileState,
    now: DateTime<Utc>,
    content_hash: Option<String>,
) -> Result<file::Model, DbErr> {
    let version = file.version + 1;
    let mut file = file.into_active_model();
    file.state = Set(state);
    file.version = Set(version);
    file.updated_at = Set(now);
    file.search_reconciled_at = Set(None);
    if let Some(hash) = content_hash {
        file.content_hash = Set(Some(hash));
    }
    file.update(transaction).await
}

pub async fn list_ready(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
    page: PageRequest,
) -> Result<Vec<file::Model>, DbErr> {
    let mut query = file::Entity::find()
        .filter(file::Column::OrganizationId.eq(organization_id))
        .filter(file::Column::State.eq(FileState::Ready));
    if let Some(after_id) = page.after_id {
        query = query.filter(file::Column::Id.gt(after_id));
    }
    query
        .order_by_asc(file::Column::Id)
        .limit(page.limit + 1)
        .all(connection)
        .await
}

pub async fn ready_candidates(
    connection: &impl ConnectionTrait,
    organization_id: Uuid,
    candidate_ids: Vec<Uuid>,
) -> Result<Vec<file::Model>, DbErr> {
    file::Entity::find()
        .filter(file::Column::OrganizationId.eq(organization_id))
        .filter(file::Column::State.eq(FileState::Ready))
        .filter(file::Column::Id.is_in(candidate_ids))
        .all(connection)
        .await
}

pub async fn record_search_reconciliation(
    transaction: &DatabaseTransaction,
    file_id: Uuid,
    version: i64,
    now: DateTime<Utc>,
) -> Result<bool, DbErr> {
    Ok(file::Entity::update_many()
        .col_expr(file::Column::SearchReconciledAt, Expr::value(Some(now)))
        .col_expr(
            file::Column::SearchDocumentMayExist,
            Expr::col(file::Column::State).eq(FileState::Ready),
        )
        .filter(file::Column::Id.eq(file_id))
        .filter(file::Column::Version.eq(version))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}
