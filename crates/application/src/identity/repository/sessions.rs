use super::super::entities::session;
use crate::pagination::PageRequest;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    ExprTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    sea_query::{Expr, Func},
};
use uuid::Uuid;

pub async fn find_by_hash(
    connection: &impl ConnectionTrait,
    token_hash: &str,
) -> Result<Option<session::Model>, DbErr> {
    session::Entity::find()
        .filter(session::Column::TokenHash.eq(token_hash))
        .filter(Expr::col(session::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .filter(Expr::col(session::Column::IdleExpiresAt).gt(Func::cust("clock_timestamp")))
        .one(connection)
        .await
}
pub async fn find_by_id(
    connection: &impl ConnectionTrait,
    session_id: Uuid,
) -> Result<Option<session::Model>, DbErr> {
    session::Entity::find_by_id(session_id)
        .filter(Expr::col(session::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .filter(Expr::col(session::Column::IdleExpiresAt).gt(Func::cust("clock_timestamp")))
        .one(connection)
        .await
}
pub async fn insert_session(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    token_hash: String,
    security_version: i64,
    expires_at: DateTime<Utc>,
    idle_expires_at: DateTime<Utc>,
) -> Result<session::Model, DbErr> {
    session::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(account_id),
        token_hash: Set(token_hash),
        security_version: Set(security_version),
        expires_at: Set(expires_at),
        idle_expires_at: Set(idle_expires_at),
        ..Default::default()
    }
    .insert(transaction)
    .await
}
pub async fn rotate_session(
    transaction: &DatabaseTransaction,
    session_id: Uuid,
    token_hash: String,
) -> Result<(), DbErr> {
    session::Entity::update_many()
        .col_expr(session::Column::TokenHash, Expr::value(token_hash))
        .filter(session::Column::Id.eq(session_id))
        .exec(transaction)
        .await?;
    Ok(())
}
pub async fn touch_session(
    transaction: &DatabaseTransaction,
    session_id: Uuid,
    now: DateTime<Utc>,
    idle_expires_at: DateTime<Utc>,
) -> Result<(), DbErr> {
    session::Entity::update_many()
        .col_expr(session::Column::LastSeenAt, Expr::value(now))
        .col_expr(session::Column::IdleExpiresAt, Expr::value(idle_expires_at))
        .filter(session::Column::Id.eq(session_id))
        .filter(Expr::col(session::Column::IdleExpiresAt).gt(Func::cust("clock_timestamp")))
        .exec(transaction)
        .await?;
    Ok(())
}
pub async fn revoke_session(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    session_id: Uuid,
) -> Result<bool, DbErr> {
    Ok(session::Entity::delete_many()
        .filter(session::Column::AccountId.eq(account_id))
        .filter(session::Column::Id.eq(session_id))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}
pub async fn revoke_all(transaction: &DatabaseTransaction, account_id: Uuid) -> Result<(), DbErr> {
    session::Entity::delete_many()
        .filter(session::Column::AccountId.eq(account_id))
        .exec(transaction)
        .await?;
    Ok(())
}
pub async fn list_for_account(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
    page: PageRequest,
) -> Result<Vec<session::Model>, DbErr> {
    let mut query = session::Entity::find()
        .filter(session::Column::AccountId.eq(account_id))
        .filter(Expr::col(session::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .filter(Expr::col(session::Column::IdleExpiresAt).gt(Func::cust("clock_timestamp")));
    if let Some(after_id) = page.after_id {
        query = query.filter(session::Column::Id.gt(after_id));
    }
    query
        .order_by_asc(session::Column::Id)
        .limit(page.limit + 1)
        .all(connection)
        .await
}
