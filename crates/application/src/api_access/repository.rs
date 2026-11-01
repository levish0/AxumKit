use super::entities::{api_client, api_key, api_usage};
use crate::{ApplicationResult, pagination::PageRequest};
use chrono::{DateTime, Duration, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, sea_query::LockType,
};
use uuid::Uuid;

pub async fn lock_client(
    transaction: &DatabaseTransaction,
    id: Uuid,
) -> Result<Option<api_client::Model>, sea_orm::DbErr> {
    api_client::Entity::find_by_id(id)
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}
pub async fn find_key(
    connection: &impl ConnectionTrait,
    id: Uuid,
) -> Result<Option<api_key::Model>, sea_orm::DbErr> {
    api_key::Entity::find_by_id(id).one(connection).await
}
pub async fn lock_key(
    transaction: &DatabaseTransaction,
    id: Uuid,
) -> Result<Option<api_key::Model>, sea_orm::DbErr> {
    api_key::Entity::find_by_id(id)
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}
pub async fn client_count(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<u64, sea_orm::DbErr> {
    api_client::Entity::find()
        .filter(api_client::Column::OrganizationId.eq(organization_id))
        .count(transaction)
        .await
}
pub async fn key_count(
    transaction: &DatabaseTransaction,
    client_id: Uuid,
) -> Result<u64, sea_orm::DbErr> {
    api_key::Entity::find()
        .filter(api_key::Column::ClientId.eq(client_id))
        .count(transaction)
        .await
}
pub async fn list_clients(
    transaction: &DatabaseTransaction,
    organization_id: Uuid,
    page: PageRequest,
) -> Result<Vec<api_client::Model>, sea_orm::DbErr> {
    let mut query =
        api_client::Entity::find().filter(api_client::Column::OrganizationId.eq(organization_id));
    if let Some(id) = page.after_id {
        query = query.filter(api_client::Column::Id.gt(id));
    }
    query
        .order_by_asc(api_client::Column::Id)
        .limit(page.limit + 1)
        .all(transaction)
        .await
}
pub async fn list_keys(
    transaction: &DatabaseTransaction,
    client_id: Uuid,
    page: PageRequest,
) -> Result<Vec<api_key::Model>, sea_orm::DbErr> {
    let mut query = api_key::Entity::find().filter(api_key::Column::ClientId.eq(client_id));
    if let Some(id) = page.after_id {
        query = query.filter(api_key::Column::Id.gt(id));
    }
    query
        .order_by_asc(api_key::Column::Id)
        .limit(page.limit + 1)
        .all(transaction)
        .await
}
pub async fn cleanup_usage(
    transaction: &DatabaseTransaction,
    days: i64,
    now: DateTime<Utc>,
) -> ApplicationResult<u64> {
    let cutoff = now - Duration::days(days);
    let candidates = api_usage::Entity::find()
        .select_only()
        .column(api_usage::Column::Id)
        .filter(api_usage::Column::AdmittedAt.lt(cutoff))
        .order_by_asc(api_usage::Column::AdmittedAt)
        .order_by_asc(api_usage::Column::Id)
        .limit(1000)
        .into_query();
    Ok(api_usage::Entity::delete_many()
        .filter(api_usage::Column::Id.in_subquery(candidates))
        .filter(api_usage::Column::AdmittedAt.lt(cutoff))
        .exec(transaction)
        .await?
        .rows_affected)
}
