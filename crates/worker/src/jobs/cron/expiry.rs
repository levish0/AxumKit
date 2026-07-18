use chrono::Utc;
use sea_orm::sea_query::{Alias, Expr, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait, ExprTrait};

/// Deletes rows whose `expires_col` is in the past, in batches bounded by a
/// limited id subquery (keeps lock duration short). Returns the total deleted.
///
/// Shared by the expired-ACL-rule and expired-ACL-membership cleanup jobs:
/// expiry is filtered at read/evaluation time, so this only reclaims storage.
pub(super) async fn run_batched_expiry_delete<E>(
    db: &DatabaseConnection,
    entity: E,
    id_col: E::Column,
    expires_col: E::Column,
    alias: &'static str,
    label: &'static str,
    batch_size: u64,
) -> Result<u64, anyhow::Error>
where
    E: EntityTrait + Copy,
{
    let mut total_deleted = 0u64;
    let now = Utc::now();

    loop {
        let alias = Alias::new(alias);

        let candidates = Query::select()
            .column((alias.clone(), id_col))
            .from_as(entity, alias.clone())
            .and_where(Expr::col((alias.clone(), expires_col)).is_not_null())
            .and_where(Expr::col((alias, expires_col)).lt(Expr::value(now)))
            .limit(batch_size)
            .to_owned();

        let delete = Query::delete()
            .from_table(entity)
            .and_where(Expr::col(id_col).in_subquery(candidates))
            .to_owned();

        let deleted = db.execute(&delete).await?.rows_affected();
        total_deleted += deleted;
        tracing::debug!(deleted, label, "Deleted batch of expired rows");

        if deleted < batch_size {
            break;
        }
    }

    Ok(total_deleted)
}
