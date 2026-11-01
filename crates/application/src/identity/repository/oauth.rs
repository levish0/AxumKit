use super::super::entities::{external_identity, oauth_flow, oauth_native_flow};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    ExprTrait, PaginatorTrait, QueryFilter, QuerySelect, Set,
    sea_query::{Expr, Func},
};
use uuid::Uuid;

pub async fn insert_flow(
    transaction: &DatabaseTransaction,
    flow: oauth_flow::ActiveModel,
) -> Result<(), DbErr> {
    flow.insert(transaction).await?;
    Ok(())
}

pub async fn find_flow(
    connection: &impl ConnectionTrait,
    state_hash: &str,
) -> Result<Option<oauth_flow::Model>, DbErr> {
    oauth_flow::Entity::find()
        .filter(oauth_flow::Column::StateHash.eq(state_hash))
        .filter(Expr::col(oauth_flow::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .one(connection)
        .await
}

pub async fn consume_flow(
    transaction: &DatabaseTransaction,
    flow_id: Uuid,
    binding_hash: &str,
) -> Result<bool, DbErr> {
    Ok(oauth_flow::Entity::delete_many()
        .filter(oauth_flow::Column::Id.eq(flow_id))
        .filter(oauth_flow::Column::BindingHash.eq(binding_hash))
        .filter(Expr::col(oauth_flow::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}

pub async fn insert_native_flow(
    transaction: &DatabaseTransaction,
    flow: oauth_native_flow::ActiveModel,
) -> Result<(), DbErr> {
    flow.insert(transaction).await?;
    Ok(())
}

/// Deletes and returns an unexpired native flow only when it belongs to `provider`.
pub async fn consume_native_flow(
    transaction: &DatabaseTransaction,
    flow_hash: &str,
    provider: &str,
) -> Result<Option<oauth_native_flow::Model>, DbErr> {
    Ok(oauth_native_flow::Entity::delete_many()
        .filter(oauth_native_flow::Column::FlowHash.eq(flow_hash))
        .filter(oauth_native_flow::Column::Provider.eq(provider))
        .filter(Expr::col(oauth_native_flow::Column::ExpiresAt).gt(Func::cust("clock_timestamp")))
        .exec_with_returning(transaction)
        .await?
        .pop())
}

pub async fn find_identity(
    connection: &impl ConnectionTrait,
    issuer: &str,
    subject: &str,
) -> Result<Option<external_identity::Model>, DbErr> {
    external_identity::Entity::find()
        .filter(external_identity::Column::Issuer.eq(issuer))
        .filter(external_identity::Column::Subject.eq(subject))
        .one(connection)
        .await
}

pub async fn insert_identity(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
    issuer: String,
    subject: String,
) -> Result<external_identity::Model, DbErr> {
    external_identity::ActiveModel {
        id: Set(Uuid::now_v7()),
        account_id: Set(account_id),
        issuer: Set(issuer),
        subject: Set(subject),
    }
    .insert(transaction)
    .await
}

pub async fn lock_identity(
    transaction: &DatabaseTransaction,
    identity_id: Uuid,
) -> Result<Option<external_identity::Model>, DbErr> {
    external_identity::Entity::find_by_id(identity_id)
        .lock_exclusive()
        .one(transaction)
        .await
}

pub async fn count_for_account(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<u64, DbErr> {
    external_identity::Entity::find()
        .filter(external_identity::Column::AccountId.eq(account_id))
        .count(connection)
        .await
}

pub async fn delete_identity(
    transaction: &DatabaseTransaction,
    identity_id: Uuid,
    account_id: Uuid,
) -> Result<bool, DbErr> {
    Ok(external_identity::Entity::delete_many()
        .filter(external_identity::Column::Id.eq(identity_id))
        .filter(external_identity::Column::AccountId.eq(account_id))
        .exec(transaction)
        .await?
        .rows_affected
        == 1)
}

pub async fn list_identities(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<Vec<external_identity::Model>, DbErr> {
    external_identity::Entity::find()
        .filter(external_identity::Column::AccountId.eq(account_id))
        .all(connection)
        .await
}
