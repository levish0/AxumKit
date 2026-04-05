use crate::repository::user::{
    UserUpdateParams, repository_get_user_by_id, repository_update_user,
};
use crate::utils::redis_cache::get_json_and_delete;
use axumkit_errors::errors::{Errors, ServiceResult};
use chrono::Utc;
use redis::aio::ConnectionManager;
use sea_orm::{DatabaseConnection, TransactionTrait};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailVerificationData {
    pub user_id: String,
    pub email: String,
}

///
/// # Arguments
///
/// # Returns
pub async fn service_verify_email(
    conn: &DatabaseConnection,
    redis_conn: &ConnectionManager,
    token: &str,
) -> ServiceResult<()> {
    let token_key = axumkit_constants::email_verification_key(token);
    let verification_data: EmailVerificationData = get_json_and_delete(
        redis_conn,
        &token_key,
        || Errors::TokenInvalidVerification,
        |_| Errors::TokenInvalidVerification,
    )
    .await?;

    let user_id = Uuid::parse_str(&verification_data.user_id)
        .map_err(|_| Errors::TokenInvalidVerification)?;

    let txn = conn.begin().await?;

    let user = repository_get_user_by_id(&txn, user_id).await?;

    if user.verified_at.is_some() {
        return Err(Errors::EmailAlreadyVerified);
    }

    if user.email != verification_data.email {
        return Err(Errors::TokenEmailMismatch);
    }

    repository_update_user(
        &txn,
        user_id,
        UserUpdateParams {
            verified_at: Some(Some(Utc::now())),
            ..Default::default()
        },
    )
    .await?;

    txn.commit().await?;

    info!(user_id = %user_id, "Email verified");

    Ok(())
}
