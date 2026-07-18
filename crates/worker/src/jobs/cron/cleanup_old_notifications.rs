use chrono::{Duration, Utc};
use entity::{notification_deliveries, notification_events};
use sea_orm::sea_query::{Alias, Expr, Query};
use sea_orm::{ConnectionTrait, DatabaseConnection, ExprTrait};

/// Cleanup old notifications
///
/// Deletes notifications where created_at < NOW() - retention_days.
pub async fn run_cleanup_old_notifications(
    db: &DatabaseConnection,
    retention_days: u32,
    batch_size: u64,
) -> Result<u64, anyhow::Error> {
    let mut total_deleted = 0u64;
    let cutoff = Utc::now() - Duration::days(retention_days as i64);

    loop {
        let alias = Alias::new("nd");

        let candidates = Query::select()
            .column((alias.clone(), notification_deliveries::Column::Id))
            .from_as(notification_deliveries::Entity, alias.clone())
            .and_where(
                Expr::col((alias, notification_deliveries::Column::CreatedAt))
                    .lt(Expr::value(cutoff)),
            )
            .limit(batch_size)
            .to_owned();

        let delete_query = Query::delete()
            .from_table(notification_deliveries::Entity)
            .and_where(Expr::col(notification_deliveries::Column::Id).in_subquery(candidates))
            .to_owned();

        let delete_result = db.execute(&delete_query).await?;
        let deleted = delete_result.rows_affected();
        total_deleted += deleted;

        tracing::debug!(
            deleted = deleted,
            retention_days = retention_days,
            "Deleted batch of old notifications"
        );

        if deleted < batch_size {
            break;
        }
    }

    // Notification writers insert the event row and all delivery rows in a
    // single transaction, so this query only observes truly orphaned committed
    // events rather than in-flight event creation.
    let orphaned_event_delete = Query::delete()
        .from_table(notification_events::Entity)
        .and_where(
            Expr::col((notification_events::Entity, notification_events::Column::Id))
                .not_in_subquery(
                    Query::select()
                        .column(notification_deliveries::Column::EventId)
                        .from(notification_deliveries::Entity)
                        .to_owned(),
                ),
        )
        .to_owned();

    let orphaned_deleted = db.execute(&orphaned_event_delete).await?.rows_affected();
    tracing::debug!(
        deleted = orphaned_deleted,
        "Deleted orphaned notification events after delivery cleanup"
    );

    Ok(total_deleted)
}
