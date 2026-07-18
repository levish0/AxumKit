use chrono::Utc;
use entity::user_roles;
use sea_orm::sea_query::{Alias, Expr, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, ExprTrait, FromQueryResult};
use uuid::Uuid;

#[derive(Debug, FromQueryResult)]
struct RoleRow {
    id: Uuid,
    user_id: Uuid,
}

/// Cleanup expired user roles
///
/// Deletes roles where expires_at < NOW() in batches.
/// Returns (total_deleted, affected_user_ids).
pub async fn run_cleanup_expired_roles(
    db: &DatabaseConnection,
    batch_size: u64,
) -> Result<(u64, Vec<Uuid>), anyhow::Error> {
    let mut total_deleted = 0u64;
    let mut affected_user_ids = Vec::new();
    let now = Utc::now();

    loop {
        let alias = Alias::new("ur");

        let candidates_query = Query::select()
            .columns([
                (alias.clone(), user_roles::Column::Id),
                (alias.clone(), user_roles::Column::UserId),
            ])
            .from_as(user_roles::Entity, alias.clone())
            .and_where(Expr::col((alias.clone(), user_roles::Column::ExpiresAt)).is_not_null())
            .and_where(Expr::col((alias, user_roles::Column::ExpiresAt)).lt(Expr::value(now)))
            .limit(batch_size)
            .to_owned();

        let rows = RoleRow::find_by_statement(db.get_database_backend().build(&candidates_query))
            .all(db)
            .await?;

        if rows.is_empty() {
            break;
        }

        let role_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        affected_user_ids.extend(rows.iter().map(|r| r.user_id));

        // Delete using the IDs fetched by the initial SELECT
        let delete_query = Query::delete()
            .from_table(user_roles::Entity)
            .and_where(Expr::col(user_roles::Column::Id).is_in(role_ids))
            .to_owned();

        let delete_result = db.execute(&delete_query).await?;
        let deleted = delete_result.rows_affected();
        total_deleted += deleted;

        tracing::debug!(deleted = deleted, "Deleted batch of expired user roles");

        if deleted < batch_size {
            break;
        }
    }

    Ok((total_deleted, affected_user_ids))
}
