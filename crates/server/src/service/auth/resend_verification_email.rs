use crate::bridge::worker_client;
use crate::service::auth::verify_email::{
    find_pending_email_signup_by_email, reissue_pending_email_signup_token,
};
use crate::state::WorkerClient;
use errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use tracing::info;

/// Resend a verification email for a pending email/password signup.
///
/// Issues a **fresh** token (the raw token is not stored, only its hash),
/// preserving the remaining validity window and invalidating the previous link,
/// then mails it. Returns `Ok(())` silently if no pending signup exists (prevents
/// email enumeration).
pub async fn service_resend_verification_email(
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    email: &str,
) -> ServiceResult<()> {
    let Some((token_id, signup_data)) =
        find_pending_email_signup_by_email(redis_conn, email).await?
    else {
        return Ok(());
    };

    // Re-issue within the remaining window; None means it expired between lookup and now.
    let Some((new_token, remaining_minutes)) =
        reissue_pending_email_signup_token(redis_conn, &token_id, &signup_data).await?
    else {
        return Ok(());
    };

    worker_client::send_verification_email(
        worker,
        &signup_data.email,
        &signup_data.handle,
        &new_token,
        remaining_minutes,
    )
    .await?;

    // Do not log PII (email/handle); the pending-signup flow has no user_id to correlate on yet.
    info!("Pending signup verification email resent");

    Ok(())
}
