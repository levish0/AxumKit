use crate::repository::oauth::create_oauth_connection::repository_create_oauth_connection;
use crate::repository::oauth::create_oauth_user::repository_create_oauth_user;
use crate::repository::oauth::find_user_by_oauth::repository_find_user_by_oauth;
use crate::repository::user::find_by_email::repository_find_user_by_email;
use crate::repository::user::find_by_handle::repository_find_user_by_handle;
use crate::service::auth::verify_email::{
    find_pending_email_signup_by_email, find_pending_email_signup_by_handle,
};
use crate::service::oauth::types::OAuthUserResult;
use entity::common::OAuthProvider;
use errors::errors::{Errors, ServiceResult};
use redis::aio::ConnectionManager;
use sea_orm::ConnectionTrait;
use tracing::info;

/// Finds or creates a user using information from an OAuth provider.
///
/// # Arguments
/// * `conn` - Database connection (must be called within a transaction)
/// * `provider` - OAuth provider (Google, GitHub, etc.)
/// * `provider_user_id` - User ID from the OAuth provider
/// * `email` - User email
/// * `display_name` - User display name
/// * `handle` - User handle (required for new user creation)
/// * `profile_image` - Profile image URL (optional)
///
/// # Returns
/// * `OAuthUserResult` - User model and whether the user is new
pub async fn service_find_or_create_oauth_user<C>(
    conn: &C,
    redis_conn: &ConnectionManager,
    provider: OAuthProvider,
    provider_user_id: &str,
    email: &str,
    display_name: &str,
    handle: Option<&str>,
    profile_image: Option<String>,
) -> ServiceResult<OAuthUserResult>
where
    C: ConnectionTrait,
{
    // 1. Check if an existing OAuth connection exists
    if let Some(user) =
        repository_find_user_by_oauth(conn, provider.clone(), provider_user_id).await?
    {
        return Ok(OAuthUserResult {
            user,
            is_new_user: false,
        });
    }

    // 2. Check if an existing account with the same email exists (security: prevent automatic linking)
    if repository_find_user_by_email(conn, email.to_string())
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 2b. Check if a pending email/password signup holds this email
    if find_pending_email_signup_by_email(redis_conn, email)
        .await?
        .is_some()
    {
        return Err(Errors::OauthEmailAlreadyExists);
    }

    // 3. New user - handle required
    let handle = handle.ok_or(Errors::OauthHandleRequired)?;

    // 3. Check handle uniqueness
    if repository_find_user_by_handle(conn, handle.to_string())
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // 3b. Check if a pending email/password signup holds this handle
    if find_pending_email_signup_by_handle(redis_conn, handle)
        .await?
        .is_some()
    {
        return Err(Errors::UserHandleAlreadyExists);
    }

    // 4. Create new user
    let new_user =
        repository_create_oauth_user(conn, email, display_name, handle, profile_image).await?;

    // 5. Create OAuth connection
    repository_create_oauth_connection(conn, &new_user.id, provider.clone(), provider_user_id)
        .await?;

    info!(user_id = %new_user.id, provider = ?provider, "OAuth user created");

    Ok(OAuthUserResult {
        user: new_user,
        is_new_user: true,
    })
}
