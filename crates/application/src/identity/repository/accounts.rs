use super::super::entities::account;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbErr, EntityTrait,
    ExprTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
    sea_query::{Expr, LockType},
};
use uuid::Uuid;

pub async fn find_by_email(
    connection: &impl ConnectionTrait,
    email: &str,
) -> Result<Option<account::Model>, DbErr> {
    account::Entity::find()
        .filter(account::Column::Email.eq(email))
        .one(connection)
        .await
}
pub async fn find_by_id(
    connection: &impl ConnectionTrait,
    account_id: Uuid,
) -> Result<Option<account::Model>, DbErr> {
    account::Entity::find_by_id(account_id)
        .one(connection)
        .await
}
pub async fn lock_account(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
) -> Result<Option<account::Model>, DbErr> {
    // Serialize account changes without blocking child foreign-key checks.
    account::Entity::find_by_id(account_id)
        .lock(LockType::NoKeyUpdate)
        .one(transaction)
        .await
}
pub async fn insert_account(
    transaction: &DatabaseTransaction,
    email: String,
    password_hash: Option<String>,
    verified: bool,
) -> Result<account::Model, DbErr> {
    account::ActiveModel {
        id: Set(Uuid::now_v7()),
        email: Set(email),
        password_hash: Set(password_hash),
        verified: Set(verified),
        ..Default::default()
    }
    .insert(transaction)
    .await
}
pub async fn mark_verified(
    transaction: &DatabaseTransaction,
    account: account::Model,
) -> Result<account::Model, DbErr> {
    let mut account = account.into_active_model();
    account.verified = Set(true);
    account.update(transaction).await
}
pub async fn replace_password(
    transaction: &DatabaseTransaction,
    account: account::Model,
    password_hash: String,
) -> Result<account::Model, DbErr> {
    let next_version = account.security_version + 1;
    let mut account = account.into_active_model();
    account.password_hash = Set(Some(password_hash));
    account.security_version = Set(next_version);
    account.update(transaction).await
}
pub async fn set_active(
    transaction: &DatabaseTransaction,
    account: account::Model,
    active: bool,
) -> Result<(), DbErr> {
    let next_version = account.security_version + 1;
    let mut account = account.into_active_model();
    account.active = Set(active);
    account.security_version = Set(next_version);
    account.update(transaction).await?;
    Ok(())
}
pub async fn set_totp(
    transaction: &DatabaseTransaction,
    account: account::Model,
    secret: Option<String>,
    last_step: i64,
) -> Result<(), DbErr> {
    let next_version = account.security_version + 1;
    let mut account = account.into_active_model();
    account.totp_secret = Set(secret);
    account.totp_last_step = Set(last_step);
    account.security_version = Set(next_version);
    account.update(transaction).await?;
    Ok(())
}
pub async fn record_totp_step(
    transaction: &DatabaseTransaction,
    account: account::Model,
    step: i64,
) -> Result<(), DbErr> {
    let mut account = account.into_active_model();
    account.totp_last_step = Set(step);
    account.update(transaction).await?;
    Ok(())
}

pub async fn advance_security_version(
    transaction: &DatabaseTransaction,
    account_id: Uuid,
) -> Result<(), DbErr> {
    account::Entity::update_many()
        .col_expr(
            account::Column::SecurityVersion,
            Expr::col(account::Column::SecurityVersion).add(1),
        )
        .filter(account::Column::Id.eq(account_id))
        .exec(transaction)
        .await?;
    Ok(())
}

pub async fn set_administrator(
    transaction: &DatabaseTransaction,
    account: account::Model,
    administrator: bool,
) -> Result<(), DbErr> {
    let next_version = account.security_version + 1;
    let mut account = account.into_active_model();
    account.administrator = Set(administrator);
    account.security_version = Set(next_version);
    account.update(transaction).await?;
    Ok(())
}
