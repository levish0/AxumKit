use super::super::entities::{audit_event, rate_limit, recovery_code};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, DbErr, EntityTrait, ExprTrait, QueryFilter,
    Set,
    sea_query::{Expr, Func, OnConflict},
};
use uuid::Uuid;

pub async fn consume_rate_limit(
    transaction: &DatabaseTransaction,
    key: String,
    expires_at: DateTime<Utc>,
) -> Result<i32, DbErr> {
    let expired = Expr::col((rate_limit::Entity, rate_limit::Column::ExpiresAt))
        .lte(Func::cust("clock_timestamp"));
    let record = rate_limit::Entity::insert(rate_limit::ActiveModel {
        id: Set(Uuid::now_v7()),
        key: Set(key),
        attempts: Set(1),
        expires_at: Set(expires_at),
    })
    .on_conflict(
        OnConflict::column(rate_limit::Column::Key)
            .value(
                rate_limit::Column::Attempts,
                Expr::case(expired.clone(), 1).finally(Func::cust("least").args([
                    Expr::col((rate_limit::Entity, rate_limit::Column::Attempts)).add(1),
                    // A shared ceiling must exceed every supported per-instance budget.
                    Expr::value(crate::config::MAX_AUTHENTICATION_ATTEMPTS_PER_MINUTE + 1),
                ])),
            )
            .value(
                rate_limit::Column::ExpiresAt,
                Expr::case(expired, expires_at).finally(Expr::col((
                    rate_limit::Entity,
                    rate_limit::Column::ExpiresAt,
                ))),
            )
            .to_owned(),
    )
    .exec_with_returning(transaction)
    .await?;
    Ok(record.attempts)
}
pub async fn append_audit(
    transaction: &DatabaseTransaction,
    account_id: Option<Uuid>,
    event: &'static str,
    request_id: Uuid,
) -> Result<(), DbErr> {
    audit_event::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(account_id),
        event: Set(event.into()),
        request_id: Set(request_id),
        ..Default::default()
    }
    .insert(transaction)
    .await?;
    Ok(())
}
pub async fn replace_recovery_codes(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    hashes: Vec<String>,
) -> Result<(), DbErr> {
    recovery_code::Entity::delete_many()
        .filter(recovery_code::Column::AccountId.eq(account_id))
        .exec(transaction)
        .await?;
    if !hashes.is_empty() {
        recovery_code::Entity::insert_many(hashes.into_iter().map(|code_hash| {
            recovery_code::ActiveModel {
                id: Set(Uuid::now_v7()),
                account_id: Set(account_id),
                code_hash: Set(code_hash),
            }
        }))
        .exec(transaction)
        .await?;
    }
    Ok(())
}
pub async fn consume_recovery_code(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    code_hash: &str,
) -> Result<bool, DbErr> {
    Ok(recovery_code::Entity::delete_many()
        .filter(recovery_code::Column::AccountId.eq(account_id))
        .filter(recovery_code::Column::CodeHash.eq(code_hash))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}
