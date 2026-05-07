use crate::bridge::worker_client;
use crate::connection::R2Client;
use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::create_oauth_user::repository_create_oauth_user;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use crate::service::auth::session::SessionService;
use crate::service::auth::verify_email::{
    find_pending_email_signup_by_email, find_pending_email_signup_by_handle,
};
use crate::service::oauth::download_profile_image::download_and_upload_profile_image;
use crate::service::oauth::types::PendingSignupData;
use crate::state::WorkerClient;
use crate::utils::redis_cache::delete_key;
use constants::{oauth_pending_key, oauth_pending_lock_key};
use errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use reqwest::Client as HttpClient;
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};
use std::sync::LazyLock;
use tracing::{info, warn};
use uuid::Uuid;

const OAUTH_PENDING_LOCK_TTL_SECONDS: u64 = 60;
static RELEASE_PENDING_LOCK_SCRIPT: LazyLock<redis::Script> =
    LazyLock::new(|| redis::Script::new(include_str!("lua/release_pending_lock.lua")));

/// Complete OAuth pending signup and create a session.
pub async fn service_complete_signup<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    http_client: &HttpClient,
    r2_assets: &R2Client,
    worker: &WorkerClient,
    pending_token: &str,
    handle: &str,
    display_name: &str,
    anonymous_user_id: &str,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<String>
where
    C: ConnectionTrait + TransactionTrait,
{
    // 1. Acquire per-token lock so only one completion attempt runs at a time.
    let pending_key = oauth_pending_key(pending_token);
    let lock_key = oauth_pending_lock_key(pending_token);
    let lock_token = Uuid::now_v7().to_string();

    if !try_acquire_pending_lock(redis_conn, &lock_key, &lock_token).await? {
        return Err(Errors::BadRequestError(
            "OAuth signup is already in progress. Please retry shortly.".to_string(),
        ));
    }

    let complete_result = async {
        // 2. Read pending payload without consuming token yet.
        let mut lock_conn = redis_conn.clone();
        let pending_json: Option<String> = lock_conn.get(&pending_key).await.map_err(|e| {
            Errors::SysInternalError(format!(
                "Redis read failed for key '{}': {}",
                pending_key, e
            ))
        })?;

        let pending_json = pending_json.ok_or(Errors::UserTokenExpired)?;
        let pending_data: PendingSignupData =
            serde_json::from_str(&pending_json).map_err(|_| Errors::UserInvalidToken)?;

        // Bind pending token to the same anonymous browser context used in login flow.
        if pending_data.anonymous_user_id != anonymous_user_id {
            return Err(Errors::UserInvalidToken);
        }

        let provider = pending_data.provider.clone();
        let provider_user_id = pending_data.provider_user_id.clone();
        let email = pending_data.email.clone();

        // 3. Pre-check duplicates before any external I/O.
        if repository_find_user_by_oauth(conn, provider.clone(), &provider_user_id)
            .await?
            .is_some()
        {
            return Err(Errors::OauthAccountAlreadyLinked);
        }

        if repository_find_user_by_email(conn, email.clone())
            .await?
            .is_some()
        {
            return Err(Errors::OauthEmailAlreadyExists);
        }

        // Check if a pending email/password signup holds this email
        if find_pending_email_signup_by_email(redis_conn, &email)
            .await?
            .is_some()
        {
            return Err(Errors::OauthEmailAlreadyExists);
        }

        if repository_find_user_by_handle(conn, handle.to_string())
            .await?
            .is_some()
        {
            return Err(Errors::UserHandleAlreadyExists);
        }

        // Check if a pending email/password signup holds this handle
        if find_pending_email_signup_by_handle(redis_conn, handle)
            .await?
            .is_some()
        {
            return Err(Errors::UserHandleAlreadyExists);
        }

        // 4. External I/O stays outside transaction.
        let profile_image_key = match pending_data.profile_image {
            Some(ref url) => download_and_upload_profile_image(http_client, r2_assets, url).await,
            None => None,
        };

        let create_result = async {
            let txn = conn.begin().await?;

            // Re-check inside transaction to reduce race windows.
            if repository_find_user_by_oauth(&txn, provider.clone(), &provider_user_id)
                .await?
                .is_some()
            {
                return Err(Errors::OauthAccountAlreadyLinked);
            }

            if repository_find_user_by_email(&txn, email.clone())
                .await?
                .is_some()
            {
                return Err(Errors::OauthEmailAlreadyExists);
            }

            if repository_find_user_by_handle(&txn, handle.to_string())
                .await?
                .is_some()
            {
                return Err(Errors::UserHandleAlreadyExists);
            }

            let new_user = repository_create_oauth_user(
                &txn,
                &pending_data.email,
                display_name,
                handle,
                profile_image_key.clone(),
            )
            .await?;

            repository_create_oauth_connection(
                &txn,
                &new_user.id,
                provider.clone(),
                &provider_user_id,
            )
            .await?;

            txn.commit().await?;
            Ok(new_user)
        }
        .await;

        let new_user = match create_result {
            Ok(new_user) => new_user,
            Err(err) => {
                if let Some(ref key) = profile_image_key {
                    if let Err(cleanup_err) = r2_assets.delete(key).await {
                        warn!(
                            storage_key = %key,
                            error = ?cleanup_err,
                            "Failed to cleanup uploaded OAuth profile image after DB failure"
                        );
                    }
                }
                return Err(err);
            }
        };

        // 5. Consume pending token only after successful signup.
        if let Err(err) = delete_key(redis_conn, &pending_key).await {
            warn!(
                pending_key = %pending_key,
                error = ?err,
                "Failed to consume OAuth pending signup token after successful completion"
            );
        }

        info!(
            user_id = %new_user.id,
            provider = ?provider,
            "OAuth signup completed"
        );

        // 6. Async side effects after commit.
        worker_client::index_user(worker, new_user.id).await.ok();

        let session = SessionService::create_session(
            redis_conn,
            new_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        Ok(session.session_id)
    }
    .await;

    if let Err(lock_err) = release_pending_lock(redis_conn, &lock_key, &lock_token).await {
        warn!(
            lock_key = %lock_key,
            error = ?lock_err,
            "Failed to release OAuth pending signup lock"
        );
    }

    complete_result
}

async fn try_acquire_pending_lock(
    redis_conn: &ConnectionManager,
    lock_key: &str,
    lock_token: &str,
) -> Result<bool, Errors> {
    let mut conn = redis_conn.clone();
    let result: Option<String> = redis::cmd("SET")
        .arg(lock_key)
        .arg(lock_token)
        .arg("NX")
        .arg("EX")
        .arg(OAUTH_PENDING_LOCK_TTL_SECONDS)
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            Errors::SysInternalError(format!(
                "Failed to acquire OAuth pending signup lock '{}': {}",
                lock_key, e
            ))
        })?;

    Ok(matches!(result, Some(value) if value == "OK"))
}

async fn release_pending_lock(
    redis_conn: &ConnectionManager,
    lock_key: &str,
    lock_token: &str,
) -> Result<(), Errors> {
    let mut conn = redis_conn.clone();
    let _: i32 = RELEASE_PENDING_LOCK_SCRIPT
        .key(lock_key)
        .arg(lock_token)
        .invoke_async(&mut conn)
        .await
        .map_err(|e| {
            Errors::SysInternalError(format!(
                "Failed to release OAuth pending signup lock '{}': {}",
                lock_key, e
            ))
        })?;

    Ok(())
}
