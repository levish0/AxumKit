use super::super::entities::challenge;
use crate::identity::ChallengePurpose;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    ExprTrait, QueryFilter, Set,
    sea_query::{Expr, Func},
};
use uuid::Uuid;

pub async fn find_valid(
    connection: &impl ConnectionTrait,
    token_hash: &str,
    purpose: ChallengePurpose,
) -> Result<Option<challenge::Model>, DbErr> {
    challenge::Entity::find()
        .filter(challenge::Column::TokenHash.eq(token_hash))
        .filter(challenge::Column::Purpose.eq(purpose))
        .filter(Expr::col(challenge::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .one(connection)
        .await
}
pub async fn insert_challenge(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    token_hash: String,
    purpose: ChallengePurpose,
    security_version: i64,
    expires_at: DateTime<Utc>,
    secret: Option<String>,
) -> Result<(), DbErr> {
    challenge::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(account_id),
        token_hash: Set(token_hash),
        purpose: Set(purpose),
        security_version: Set(security_version),
        expires_at: Set(expires_at),
        secret: Set(secret),
        ..Default::default()
    }
    .insert(transaction)
    .await?;
    Ok(())
}
pub async fn delete_challenge(
    transaction: &DatabaseTransaction,
    challenge_id: Uuid,
) -> Result<bool, DbErr> {
    Ok(challenge::Entity::delete_many()
        .filter(challenge::Column::Id.eq(challenge_id))
        .filter(Expr::col(challenge::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}
pub async fn delete_for_account(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    purpose: Option<ChallengePurpose>,
) -> Result<(), DbErr> {
    let mut query =
        challenge::Entity::delete_many().filter(challenge::Column::AccountId.eq(account_id));
    if let Some(purpose) = purpose {
        query = query.filter(challenge::Column::Purpose.eq(purpose));
    }
    query.exec(transaction).await?;
    Ok(())
}
