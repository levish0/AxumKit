use super::publish_job;
use crate::state::WorkerClient;
use errors::errors::Errors;
use job_queue::jobs::email::{EmailTemplate, SendEmailJob};
use job_queue::subjects::EMAIL_SUBJECT;
use tracing::info;

/// Push a verification email job to the worker queue
pub async fn send_verification_email(
    worker: &WorkerClient,
    email_to: &str,
    username: &str,
    verification_token: &str,
    valid_minutes: u64,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: email_to.to_string(),
        subject: "Verify your email".to_string(),
        template: EmailTemplate::Verification {
            username: username.to_string(),
            token: verification_token.to_string(),
            valid_minutes,
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(template = "verification", "Verification email job queued");
    Ok(())
}

/// Push a password reset email job to the worker queue
pub async fn send_password_reset_email(
    worker: &WorkerClient,
    email_to: &str,
    handle: &str,
    reset_token: &str,
    valid_minutes: u64,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: email_to.to_string(),
        subject: "Reset your password".to_string(),
        template: EmailTemplate::PasswordReset {
            handle: handle.to_string(),
            token: reset_token.to_string(),
            valid_minutes,
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(
        template = "password_reset",
        "Password reset email job queued"
    );
    Ok(())
}

/// Push an email change verification job to the worker queue
pub async fn send_email_change_verification(
    worker: &WorkerClient,
    new_email: &str,
    username: &str,
    token: &str,
    valid_minutes: u64,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: new_email.to_string(),
        subject: "Confirm your email change".to_string(),
        template: EmailTemplate::EmailChange {
            username: username.to_string(),
            token: token.to_string(),
            valid_minutes,
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(
        template = "email_change",
        "Email change verification job queued"
    );
    Ok(())
}

/// Push a security-alert notification to the worker queue (OWASP ASVS 6.3.7).
///
/// Sent after a sensitive account change (password change, 2FA disable). `event` is a short
/// human-readable description; the email carries no action link — it is a notification.
pub async fn send_security_alert(
    worker: &WorkerClient,
    email_to: &str,
    username: &str,
    event: &str,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: email_to.to_string(),
        subject: "Security alert".to_string(),
        template: EmailTemplate::SecurityAlert {
            username: username.to_string(),
            event: event.to_string(),
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(
        template = "security_alert",
        "Security alert email job queued"
    );
    Ok(())
}

/// Push a new-device sign-in verification job to the worker queue.
///
/// Sent when a login succeeds from an unrecognized device (OWASP ASVS 6.3.5): the raw token
/// reaches the user only via this email, and the session is withheld until it is confirmed.
pub async fn send_device_verification(
    worker: &WorkerClient,
    email_to: &str,
    username: &str,
    device: &str,
    token: &str,
    valid_minutes: u64,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: email_to.to_string(),
        subject: "Verify your new sign-in".to_string(),
        template: EmailTemplate::DeviceVerification {
            username: username.to_string(),
            device: device.to_string(),
            token: token.to_string(),
            valid_minutes,
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(
        template = "device_verification",
        "Device verification email job queued"
    );
    Ok(())
}

/// Push an account deletion confirmation job to the worker queue.
///
/// Used to re-authenticate OAuth-only accounts (no password or TOTP factor) before
/// an irreversible account deletion: the raw token reaches the user only via this email.
pub async fn send_account_deletion_confirmation(
    worker: &WorkerClient,
    email_to: &str,
    username: &str,
    token: &str,
    valid_minutes: u64,
) -> Result<(), Errors> {
    let job = SendEmailJob {
        to: email_to.to_string(),
        subject: "Confirm your account deletion".to_string(),
        template: EmailTemplate::AccountDeletion {
            username: username.to_string(),
            token: token.to_string(),
            valid_minutes,
        },
    };

    publish_job(worker, EMAIL_SUBJECT, &job).await?;

    info!(
        template = "account_deletion",
        "Account deletion confirmation job queued"
    );
    Ok(())
}
