use crate::bridge::worker_client;
use crate::repository::user::repository_find_user_by_email;
use crate::state::WorkerClient;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Password reset token data stored in Redis
#[derive(Debug, Serialize, Deserialize)]
pub struct PasswordResetData {
    pub user_id: String,
}

/// Sends a password reset email.
///
/// Security: always returns success regardless of whether the email exists.
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

    // 1. Look up the user by email
    let user = repository_find_user_by_email(conn, email.to_string()).await?;

    // 2. Return silently if the user doesn't exist (avoids revealing email existence)
    let user = match user {
        Some(u) => u,
        None => {
            info!("Password reset requested for non-existent email");
            return Ok(());
        }
    };

    // 3. Return silently for users with no password set
    if user.password.is_none() {
        info!("Password reset requested for user without password");
        return Ok(());
    }

    // 4. Generate the token

    let reset_data = PasswordResetData {
        user_id: user.id.to_string(),
    };

    // 5. Store the token in Redis (convert minutes to seconds)
    let ttl_seconds = (config.auth_password_reset_token_expire_time * 60) as u64;
    // Store under the hashed token id so the raw token never lives in Redis;
    // the raw token is returned and only ever sent in the email link.
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        |token| constants::password_reset_key(&hash_token(token)),
        &reset_data,
        ttl_seconds,
    )
    .await?;

    // 6. Ask the Worker service to send the email
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
