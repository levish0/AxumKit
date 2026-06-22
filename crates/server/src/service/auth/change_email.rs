use crate::bridge::worker_client;
use crate::repository::user::{repository_find_user_by_email, repository_get_user_by_id};
use crate::state::WorkerClient;
use crate::utils::crypto::password::verify_password;
use crate::utils::crypto::token::{generate_secure_token, hash_token};
use crate::utils::email::normalize_email;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use config::ServerConfig;
use dto::auth::request::ChangeEmailRequest;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailChangeData {
    pub user_id: String,
    pub new_email: String,
}

pub async fn service_change_email<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    user_id: Uuid,
    payload: ChangeEmailRequest,
) -> ServiceResult<()>
where
    C: ConnectionTrait,
{
    let config = ServerConfig::get();

    let user = repository_get_user_by_id(conn, user_id).await?;

    let password_hash = user.password.ok_or(Errors::UserPasswordNotSet)?;
    verify_password(&payload.password, &password_hash)?;

    // Canonicalize once so the comparison and storage match the repository, which
    // normalizes email on lookup/write.
    let new_email = normalize_email(&payload.new_email);

    // Reject if unchanged (case/whitespace-insensitive).
    if user.email == new_email {
        return Err(Errors::BadRequestError(
            "New email must be different from current email.".to_string(),
        ));
    }

    if repository_find_user_by_email(conn, new_email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::UserEmailAlreadyExists);
    }

    let change_data = EmailChangeData {
        user_id: user.id.to_string(),
        new_email,
    };

    let ttl_seconds = (config.auth_email_change_token_expire_time * 60) as u64;
    // Store under the hashed token id so the raw token never lives in Redis;
    // the raw token is returned and only ever sent in the email link.
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        |token| constants::email_change_key(&hash_token(token)),
        &change_data,
        ttl_seconds,
    )
    .await?;

    worker_client::send_email_change_verification(
        worker,
        &change_data.new_email,
        &user.handle,
        &token,
        config.auth_email_change_token_expire_time as u64,
    )
    .await?;

    info!(user_id = %user_id, "Email change verification sent");

    Ok(())
}
