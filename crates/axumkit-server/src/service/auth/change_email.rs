use crate::bridge::worker_client;
use crate::repository::user::{repository_find_user_by_email, repository_get_user_by_id};
use crate::state::WorkerClient;
use crate::utils::crypto::password::verify_password;
use crate::utils::crypto::token::generate_secure_token;
use crate::utils::redis_cache::issue_token_and_store_json_with_ttl;
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use axumkit_config::ServerConfig;
use axumkit_dto::auth::request::ChangeEmailRequest;
use axumkit_errors::errors::{Errors, ServiceResult};

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

    if user.email == payload.new_email {
        return Err(Errors::BadRequestError(
            "New email must be different from current email.".to_string(),
        ));
    }

    if repository_find_user_by_email(conn, payload.new_email.clone())
        .await?
        .is_some()
    {
        return Err(Errors::UserEmailAlreadyExists);
    }


    let change_data = EmailChangeData {
        user_id: user.id.to_string(),
        new_email: payload.new_email.clone(),
    };

    let ttl_seconds = (config.auth_email_change_token_expire_time * 60) as u64;
    let token = issue_token_and_store_json_with_ttl(
        redis_conn,
        generate_secure_token,
        axumkit_constants::email_change_key,
        &change_data,
        ttl_seconds,
    )
    .await?;

    worker_client::send_email_change_verification(
        worker,
        &payload.new_email,
        &user.handle,
        &token,
        config.auth_email_change_token_expire_time as u64,
    )
    .await?;

    info!(user_id = %user_id, "Email change verification sent");

    Ok(())
}

