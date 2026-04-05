use crate::bridge::worker_client;
use crate::service::auth::verify_email::{
    find_pending_email_signup_by_email, get_pending_signup_remaining_minutes,
};
use crate::state::WorkerClient;
use axumkit_errors::errors::ServiceResult;
use redis::aio::ConnectionManager;
use tracing::info;

/// Resend a verification email for a pending email/password signup.
///
/// Re-sends the **existing** token with its actual remaining TTL so the email
/// template shows the real validity window. Returns `Ok(())` silently if no
/// pending signup exists (prevents email enumeration).
pub async fn service_resend_verification_email(
    redis_conn: &ConnectionManager,
    worker: &WorkerClient,
    email: &str,
) -> ServiceResult<()> {
    let Some((existing_token, signup_data)) =
        find_pending_email_signup_by_email(redis_conn, email).await?
    else {
        return Ok(());
    };

    let remaining_minutes =
        get_pending_signup_remaining_minutes(redis_conn, &existing_token).await?;

    if remaining_minutes == 0 {
        return Ok(());
    }

    worker_client::send_verification_email(
        worker,
        &signup_data.email,
        &signup_data.handle,
        &existing_token,
        remaining_minutes,
    )
    .await?;

    info!(email = %signup_data.email, handle = %signup_data.handle, "Pending signup verification email resent");

    Ok(())
}
