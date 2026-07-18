use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::create_oauth_user::repository_create_oauth_user;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use crate::service::auth::session::SessionService;
use crate::service::auth::verify_email::{
    find_pending_email_signup_by_email, find_pending_email_signup_by_handle,
};
use crate::service::oauth::types::PendingSignupTokenState;
use crate::service::user::utils::{spawn_index_user, spawn_oauth_profile_image};
use crate::state::WorkerClient;
use crate::utils::crypto::token::hash_token;
use crate::utils::redis_cache::set_json_with_ttl;
use constants::{oauth_pending_key, oauth_pending_lock_key};
use errors::errors::{Errors, ServiceResult};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};
use std::sync::LazyLock;
use tracing::{info, warn};
use uuid::Uuid;

const OAUTH_PENDING_LOCK_TTL_SECONDS: u64 = 60;
const OAUTH_COMPLETED_SIGNUP_TTL_SECONDS: u64 = 600;
static RELEASE_PENDING_LOCK_SCRIPT: LazyLock<redis::Script> =
    LazyLock::new(|| redis::Script::new(include_str!("lua/release_pending_lock.lua")));

/// Complete OAuth pending signup and create a session.
pub async fn service_complete_signup<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    pending_token: &str,
    handle: &str,
    display_name: &str,
    // Caller's anonymous browser context. `Some` for the browser flow; `None` for the native-app
    // flow (no cookie jar). Only enforced when the pending token carries a `Some` binding.
    caller_anonymous_user_id: Option<&str>,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> ServiceResult<String>
where
    C: ConnectionTrait + TransactionTrait,
{
    // 1. Acquire per-token lock so only one completion attempt runs at a time.
    // Keys are derived from the token's hash (the raw token never lives at rest in Redis; it is
    // stored hashed in resolve_oauth_sign_in — issue #133 parity).
    let token_hash = hash_token(pending_token);
    let pending_key = oauth_pending_key(&token_hash);
    let lock_key = oauth_pending_lock_key(&token_hash);
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
        let token_state: PendingSignupTokenState =
            serde_json::from_str(&pending_json).map_err(|_| Errors::UserInvalidToken)?;

        let pending_data = match token_state {
            PendingSignupTokenState::Completed {
                user_id,
                anonymous_user_id: token_anonymous_user_id,
                ..
            } => {
                if !signup_binding_matches(&token_anonymous_user_id, caller_anonymous_user_id) {
                    return Err(Errors::UserInvalidToken);
                }

                let (raw_token, _session) = SessionService::create_session(
                    redis_conn,
                    user_id.to_string(),
                    user_agent,
                    ip_address,
                )
                .await?;

                return Ok(raw_token);
            }
            PendingSignupTokenState::Pending { data } => data,
        };

        // Bind pending token to the same anonymous browser context used in the login flow
        // (browser flow only; the native-app flow stores `None` and relies on token secrecy).
        if !signup_binding_matches(&pending_data.anonymous_user_id, caller_anonymous_user_id) {
            return Err(Errors::UserInvalidToken);
        }

        let provider = pending_data.provider.clone();
        let provider_user_id = pending_data.provider_user_id.clone();
        let email = pending_data.email.clone();
        // Carry the originating binding forward so a post-commit retry re-checks the same way.
        let pending_binding = pending_data.anonymous_user_id.clone();

        // If the DB commit succeeded but the response was lost before the token
        // state was updated, recover by treating the token as completed.
        // Soft-deleted accounts have their connections removed, so this should not resolve one;
        // the deleted_at guard is defense-in-depth against a lingering connection.
        if let Some(existing_user) =
            repository_find_user_by_oauth(conn, provider.clone(), &provider_user_id).await?
            && existing_user.deleted_at.is_none()
        {
            store_completed_signup_state(
                redis_conn,
                &pending_key,
                existing_user.id,
                provider.clone(),
                provider_user_id.clone(),
                pending_binding.clone(),
            )
            .await;

            let (raw_token, _session) = SessionService::create_session(
                redis_conn,
                existing_user.id.to_string(),
                user_agent,
                ip_address,
            )
            .await?;

            return Ok(raw_token);
        }

        // 3. Pre-check duplicates before the transaction.
        if repository_find_user_by_email(conn, email.clone())
            .await?
            .is_some()
        {
            return Err(Errors::OauthEmailAlreadyExists);
        }

        // Also reject if a pending email/password signup already holds this email.
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

        // Also reject if a pending email/password signup already holds this handle.
        if find_pending_email_signup_by_handle(redis_conn, handle)
            .await?
            .is_some()
        {
            return Err(Errors::UserHandleAlreadyExists);
        }

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

            let new_user =
                repository_create_oauth_user(&txn, &pending_data.email, display_name, handle, None)
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
            Err(err) => return Err(err),
        };

        // 5. Mark the token completed briefly so retries can issue a session.
        store_completed_signup_state(
            redis_conn,
            &pending_key,
            new_user.id,
            provider.clone(),
            provider_user_id.clone(),
            pending_binding.clone(),
        )
        .await;

        info!(
            user_id = %new_user.id,
            provider = ?provider,
            "OAuth signup completed"
        );

        // 6. Async side effects after commit.
        spawn_index_user(worker, new_user.id);
        if let Some(profile_image_url) = pending_data.profile_image {
            spawn_oauth_profile_image(worker, new_user.id, profile_image_url);
        }

        let (raw_token, _session) = SessionService::create_session(
            redis_conn,
            new_user.id.to_string(),
            user_agent,
            ip_address,
        )
        .await?;

        Ok(raw_token)
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

/// Whether a caller may complete a pending signup, enforcing that the **completion channel
/// matches the origination channel**.
///
/// - `Some(expected)` — created by a browser flow: requires the same anonymous browser context
///   (an extra CSRF defense for the redirect dance). A native-app caller (`None`) cannot strip it.
/// - `None` — created by a native-app `provider/token` flow: completable **only** by a native-app
///   caller (`None`); the single-use pending token is itself the binding. Refusing a browser
///   caller here keeps the browser endpoint's CSRF defense mandatory (it never silently completes
///   an unbound pending and sets a session cookie).
fn signup_binding_matches(stored: &Option<String>, caller: Option<&str>) -> bool {
    match (stored, caller) {
        (Some(expected), Some(provided)) => expected == provided,
        (None, None) => true,
        _ => false,
    }
}

async fn store_completed_signup_state(
    redis_conn: &ConnectionManager,
    pending_key: &str,
    user_id: Uuid,
    provider: entity::common::OAuthProvider,
    provider_user_id: String,
    anonymous_user_id: Option<String>,
) {
    let completed_state = PendingSignupTokenState::Completed {
        user_id,
        provider,
        provider_user_id,
        anonymous_user_id,
    };

    if let Err(err) = set_json_with_ttl(
        redis_conn,
        pending_key,
        &completed_state,
        OAUTH_COMPLETED_SIGNUP_TTL_SECONDS,
    )
    .await
    {
        warn!(
            pending_key = %pending_key,
            error = ?err,
            "Failed to mark OAuth pending signup token completed"
        );
    }
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

#[cfg(test)]
mod tests {
    use super::signup_binding_matches;

    #[test]
    fn browser_pending_requires_matching_anonymous_context() {
        assert!(signup_binding_matches(
            &Some("anon-a".to_string()),
            Some("anon-a")
        ));
        assert!(!signup_binding_matches(
            &Some("anon-a".to_string()),
            Some("anon-b")
        ));
    }

    #[test]
    fn browser_pending_cannot_be_completed_by_app_caller() {
        // A None caller (app endpoint) must not strip a browser pending's binding.
        assert!(!signup_binding_matches(&Some("anon-a".to_string()), None));
    }

    #[test]
    fn app_pending_only_completable_by_app_caller() {
        // App-originated (None) pending is bound by token secrecy and the app channel only.
        assert!(signup_binding_matches(&None, None));
        // A browser caller (Some) must NOT complete an app pending — keeps the browser
        // endpoint's CSRF defense mandatory.
        assert!(!signup_binding_matches(&None, Some("anon-a")));
    }
}
