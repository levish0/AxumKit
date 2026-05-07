use crate::bridge::worker_client;
use crate::repository::user::repository_find_user_by_email;
use crate::state::WorkerClient;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetData {
    pub user_id: String,
}

///
pub async fn service_forgot_password<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    email: &str,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let config = ServerConfig::get();

    let user = repository_find_user_by_email(conn, email.to_string()).await?;

    let user = match user {
        Some(u) => u,
        None => {
            info!("Password reset requested for non-existent email");
            return Ok(());
        }
    };

    if user.password.is_none() {
        info!("Password reset requested for user without password");
        return Ok(());
    }

    let reset_data = PasswordResetData {
        user_id: user.id.to_string(),
    };

    let ttl_seconds = (config.auth_password_reset_token_expire_time * 60) as u64;
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        constants::password_reset_key,
        &reset_data,
        ttl_seconds,
    )
    .await?;

    worker_client::send_password_reset_email(
        worker,
        &user.email,
        &user.handle,
        &token,
        config.auth_password_reset_token_expire_time as u64,
    )
    .await?;

    info!("Password reset email sent");

    Ok(())
}
