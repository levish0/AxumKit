use super::{
    IdentityService,
    entities::{
        audit_event, challenge, mail_delivery, oauth_flow, oauth_native_flow, rate_limit, session,
    },
};
use crate::{ApplicationError, ApplicationResult, database::database_time};
use sea_orm::{
    ColumnTrait, EntityTrait, ExprTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    TransactionTrait,
    sea_query::{Expr, Func},
};

impl IdentityService {
    pub async fn cleanup_audit_events(&self, retention_days: i64) -> ApplicationResult<u64> {
        if !(1..=36500).contains(&retention_days) {
            return Err(ApplicationError::InvalidInput("invalid audit retention"));
        }
        let transaction = self.database.begin().await?;
        let cutoff = database_time(&transaction).await? - chrono::Duration::days(retention_days);
        let expired = audit_event::Entity::find()
            .select_only()
            .column(audit_event::Column::Id)
            .filter(audit_event::Column::CreatedAt.lt(cutoff))
            .order_by_asc(audit_event::Column::CreatedAt)
            .order_by_asc(audit_event::Column::Id)
            .limit(1000)
            .into_query();
        let deleted = audit_event::Entity::delete_many()
            .filter(audit_event::Column::Id.in_subquery(expired))
            .filter(audit_event::Column::CreatedAt.lt(cutoff))
            .exec(&transaction)
            .await?
            .rows_affected;
        transaction.commit().await?;
        Ok(deleted)
    }

    pub async fn cleanup_expired_authentication(&self) -> ApplicationResult<u64> {
        let transaction = self.database.begin().await?;
        let now = database_time(&transaction).await?;
        let sessions = session::Entity::find()
            .select_only()
            .column(session::Column::Id)
            .filter(
                sea_orm::Condition::any()
                    .add(session::Column::ExpiresAt.lte(now))
                    .add(session::Column::IdleExpiresAt.lte(now)),
            )
            .limit(1000)
            .into_query();
        let mut deleted = session::Entity::delete_many()
            .filter(session::Column::Id.in_subquery(sessions))
            // Recheck after waiting for a concurrent session activity update.
            .filter(
                sea_orm::Condition::any()
                    .add(Expr::col(session::Column::ExpiresAt).lte(Func::cust("clock_timestamp")))
                    .add(
                        Expr::col(session::Column::IdleExpiresAt)
                            .lte(Func::cust("clock_timestamp")),
                    ),
            )
            .exec(&transaction)
            .await?
            .rows_affected;
        let challenges = challenge::Entity::find()
            .select_only()
            .column(challenge::Column::Id)
            .filter(challenge::Column::ExpiresAt.lte(now))
            .limit(1000)
            .into_query();
        deleted += challenge::Entity::delete_many()
            .filter(challenge::Column::Id.in_subquery(challenges))
            .exec(&transaction)
            .await?
            .rows_affected;
        let flows = oauth_flow::Entity::find()
            .select_only()
            .column(oauth_flow::Column::Id)
            .filter(oauth_flow::Column::ExpiresAt.lte(now))
            .limit(1000)
            .into_query();
        deleted += oauth_flow::Entity::delete_many()
            .filter(oauth_flow::Column::Id.in_subquery(flows))
            .exec(&transaction)
            .await?
            .rows_affected;
        let native_flows = oauth_native_flow::Entity::find()
            .select_only()
            .column(oauth_native_flow::Column::Id)
            .filter(oauth_native_flow::Column::ExpiresAt.lte(now))
            .limit(1000)
            .into_query();
        deleted += oauth_native_flow::Entity::delete_many()
            .filter(oauth_native_flow::Column::Id.in_subquery(native_flows))
            .exec(&transaction)
            .await?
            .rows_affected;
        let limits = rate_limit::Entity::find()
            .select_only()
            .column(rate_limit::Column::Id)
            .filter(rate_limit::Column::ExpiresAt.lte(now))
            .limit(1000)
            .into_query();
        deleted += rate_limit::Entity::delete_many()
            .filter(rate_limit::Column::Id.in_subquery(limits))
            .filter(Expr::col(rate_limit::Column::ExpiresAt).lte(Func::cust("clock_timestamp")))
            .exec(&transaction)
            .await?
            .rows_affected;
        let deliveries = mail_delivery::Entity::find()
            .select_only()
            .column(mail_delivery::Column::Id)
            .filter(mail_delivery::Column::ExpiresAt.lte(now))
            .limit(1000)
            .into_query();
        deleted += mail_delivery::Entity::delete_many()
            .filter(mail_delivery::Column::Id.in_subquery(deliveries))
            .exec(&transaction)
            .await?
            .rows_affected;
        transaction.commit().await?;
        Ok(deleted)
    }
}
