use crate::repository::oauth::delete_oauth_connection::repository_delete_oauth_connections_for_user;
use crate::repository::user::user_roles::repository_delete_all_user_roles;
use crate::repository::user::{UserUpdateParams, repository_update_user};
use chrono::Utc;
use errors::errors::Errors;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

/// Scrubs a user's data on account deletion (soft delete) — the single source of truth for the
/// account-deletion data policy. Must be called inside a transaction (caller owns begin/commit).
///
/// # Retained (only data with a legitimate reason to survive)
/// - Scrubbed user row: `handle`/`display_name` (public identity, permanently reserved), `created_at`
/// - Actor-based content (posts/comments/...): contribution attribution
/// - `user_bans`, moderation/action logs: safety & audit
///
/// # Removed (data minimization — private data regardless of public exposure)
/// - User row PII: `email` (freed via a dummy address), `password`/`totp_*`/`bio`/images → NULL
/// - OAuth connections: so a future sign-in starts a fresh signup
/// - Roles: revoke admin/mod privileges from the deactivated account
/// - Roles: revoke admin/mod privileges from the deactivated account
pub async fn scrub_user_account<C>(conn: &C, user_id: Uuid) -> Result<(), Errors>
where
    C: ConnectionTrait,
{
    let now = Utc::now();
    let scrubbed_email = format!("deleted-{user_id}@deleted.invalid");

    // 1. Scrub the user row: free the unique email for re-registration, null the remaining PII.
    //    Keep handle/display_name for attribution and permanent reservation.
    repository_update_user(
        conn,
        user_id,
        UserUpdateParams {
            email: Some(scrubbed_email),
            bio: Some(None),
            password: Some(None),
            profile_image: Some(None),
            banner_image: Some(None),
            totp_secret: Some(None),
            totp_enabled_at: Some(None),
            totp_backup_codes: Some(None),
            deleted_at: Some(Some(now)),
            ..Default::default()
        },
    )
    .await?;

    // 2. Remove active privileges/credentials.
    repository_delete_oauth_connections_for_user(conn, user_id).await?;
    repository_delete_all_user_roles(conn, user_id).await?;

    Ok(())
}
