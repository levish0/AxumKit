use super::{
    NotificationService,
    entities::{notification, push_device},
};
use crate::{ApplicationError, ApplicationResult, database::database_time};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    TransactionTrait,
    sea_query::{Expr, Func},
};

impl NotificationService {
    pub async fn cleanup(&self, retention_days: i64) -> ApplicationResult<u64> {
        if !(1..=36500).contains(&retention_days) {
            return Err(ApplicationError::InvalidInput(
                "invalid notification retention",
            ));
        }
        let transaction = self.identity.database.begin().await?;
        let now = database_time(&transaction).await?;
        let devices = push_device::Entity::find()
            .select_only()
            .column(push_device::Column::Id)
            .filter(push_device::Column::ExpiresAt.lte(now))
            .order_by_asc(push_device::Column::ExpiresAt)
            .order_by_asc(push_device::Column::Id)
            .limit(1000)
            .into_query();
        let mut removed = push_device::Entity::delete_many()
            .filter(push_device::Column::Id.in_subquery(devices))
            .filter(Expr::col(push_device::Column::ExpiresAt).lte(Func::cust("clock_timestamp")))
            .exec(&transaction)
            .await?
            .rows_affected;
        let cutoff = now - chrono::Duration::days(retention_days);
        let notifications = notification::Entity::find()
            .select_only()
            .column(notification::Column::Id)
            .filter(notification::Column::CreatedAt.lt(cutoff))
            .order_by_asc(notification::Column::CreatedAt)
            .order_by_asc(notification::Column::Id)
            .limit(1000)
            .into_query();
        removed += notification::Entity::delete_many()
            .filter(notification::Column::Id.in_subquery(notifications))
            .filter(notification::Column::CreatedAt.lt(cutoff))
            .exec(&transaction)
            .await?
            .rows_affected;
        transaction.commit().await?;
        Ok(removed)
    }
}
