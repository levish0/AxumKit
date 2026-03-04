use crate::repository::user::{
    UserUpdateParams, repository_find_user_by_email, repository_update_user,
};
use crate::service::auth::change_email::EmailChangeData;
use crate::utils::redis_cache::get_json_and_delete;
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use tracing::info;
use uuid::Uuid;
use axumkit_errors::errors::{Errors, ServiceResult};

pub async fn service_confirm_email_change(
    conn: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<()> {
    let token_key = axumkit_constants::email_change_key(token);
    let change_data: EmailChangeData = get_json_and_delete(
        redis_conn,
        &token_key,
        || Errors::TokenInvalidEmailChange,
        |_| Errors::TokenInvalidEmailChange,
    )
    .await?;

    let user_id =
        Uuid::parse_str(&change_data.user_id).map_err(|_| Errors::TokenInvalidEmailChange)?;

    let txn = conn.begin().await?;

    if let Some(existing) =
        repository_find_user_by_email(&txn, change_data.new_email.clone()).await?
        && existing.id != user_id
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            email: Some(change_data.new_email.clone()),
            verified_at: Some(Some(chrono::Utc::now())),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "Email changed");

    Ok(())
}

