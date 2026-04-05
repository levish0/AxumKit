use crate::bridge::worker_client;
use crate::repository::user::repository_get_user_by_id;
use crate::service::auth::verify_email::EmailVerificationData;
use crate::state::WorkerClient;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use axumkit_config::ServerConfig;
use axumkit_errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use tracing::info;
use uuid::Uuid;

///
/// # Arguments
///
/// # Returns
pub async fn service_resend_verification_email<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    user_id: Uuid,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let config = ServerConfig::get();

    let user = repository_get_user_by_id(conn, user_id).await?;

    if user.verified_at.is_some() {
        return Err(Errors::EmailAlreadyVerified);
    }

    if user.password.is_none() {
        return Err(Errors::UserPasswordNotSet);
    }

    let verification_data = EmailVerificationData {
        user_id: user.id.to_string(),
        email: user.email.clone(),
    };

    let ttl_seconds = (config.auth_email_verification_token_expire_time * 60) as u64;
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        axumkit_constants::email_verification_key,
        &verification_data,
        ttl_seconds,
    )
    .await?;

    worker_client::send_verification_email(
        worker,
        &user.email,
        &user.handle,
        &token,
        config.auth_email_verification_token_expire_time as u64,
    )
    .await?;

    info!(user_id = %user_id, "Verification email resent");

    Ok(())
}
