use super::{
    IdentityService,
    entities::account,
    repository::{accounts, challenges, deliveries},
};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::{ChallengePurpose, MailDeliveryState},
};
use auth_core::token::{generate_secure_token, hash_token};
use chrono::Duration;
use jobs::{Claim, HandlerFailure, TraceContext};
use mail::{AuthenticationMail, MailPurpose, Mailer};
use sea_orm::{DatabaseTransaction, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const AUTHENTICATION_MAIL_JOB: &str = "identity.send_authentication_mail";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MailDeliveryJob {
    delivery_id: Uuid,
}

impl IdentityService {
    pub(crate) async fn register_authentication_mail(
        &self,
        transaction: &DatabaseTransaction,
        account: &account::Model,
        purpose: MailPurpose,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let token = generate_secure_token();
        let token_hash = hash_token(&token);
        let challenge_purpose = challenge_purpose(purpose).ok_or(
            ApplicationError::InvalidInput("mail purpose requires no action token"),
        )?;
        let expires_at = database_time(transaction).await? + Duration::minutes(30);
        challenges::delete_for_account(transaction, account.id, Some(challenge_purpose)).await?;
        challenges::insert_challenge(
            transaction,
            account.id,
            token_hash.clone(),
            challenge_purpose,
            account.security_version,
            expires_at,
            None,
        )
        .await?;
        let delivery_id = Uuid::now_v7();
        let mut action_url = self
            .settings
            .public_url
            .join(match purpose {
                MailPurpose::VerifyEmail => "verify-email",
                MailPurpose::ResetPassword => "reset-password",
                _ => {
                    return Err(ApplicationError::InvalidInput(
                        "invalid action mail purpose",
                    ));
                }
            })
            .map_err(ApplicationError::internal)?;
        action_url.set_fragment(Some(&format!("token={token}")));
        let mail = AuthenticationMail {
            recipient: account.email.clone(),
            application_name: self.settings.application_name.clone(),
            purpose,
            action_url: action_url.into(),
        };
        let plaintext = serde_json::to_vec(&mail).map_err(ApplicationError::internal)?;
        let ciphertext = self
            .settings
            .secret_keys
            .encrypt(&delivery_context(delivery_id), &plaintext)?;
        deliveries::insert_delivery(
            transaction,
            delivery_id,
            account.id,
            Some(token_hash),
            ciphertext,
            expires_at,
        )
        .await?;
        jobs::enqueue(
            transaction,
            Uuid::now_v7(),
            AUTHENTICATION_MAIL_JOB,
            &MailDeliveryJob { delivery_id },
            trace,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn register_account_notification(
        &self,
        transaction: &DatabaseTransaction,
        account: &account::Model,
        purpose: MailPurpose,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        if !matches!(purpose, MailPurpose::Welcome | MailPurpose::PasswordChanged) {
            return Err(ApplicationError::InvalidInput(
                "invalid account notification purpose",
            ));
        }
        let delivery_id = Uuid::now_v7();
        let mail = AuthenticationMail {
            recipient: account.email.clone(),
            application_name: self.settings.application_name.clone(),
            purpose,
            action_url: self.settings.public_url.to_string(),
        };
        let ciphertext = self.settings.secret_keys.encrypt(
            &delivery_context(delivery_id),
            &serde_json::to_vec(&mail).map_err(ApplicationError::internal)?,
        )?;
        deliveries::insert_delivery(
            transaction,
            delivery_id,
            account.id,
            None,
            ciphertext,
            database_time(transaction).await? + Duration::hours(24),
        )
        .await?;
        jobs::enqueue(
            transaction,
            Uuid::now_v7(),
            AUTHENTICATION_MAIL_JOB,
            &MailDeliveryJob { delivery_id },
            trace,
        )
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(job_id = %claim.id, request_id = %claim.request_id))]
    pub async fn execute_job(&self, claim: &Claim, mailer: &Mailer) -> Result<(), HandlerFailure> {
        if claim.kind != AUTHENTICATION_MAIL_JOB {
            return Err(HandlerFailure {
                code: "unknown_job_kind",
                retryable: false,
            });
        }
        let payload: MailDeliveryJob =
            serde_json::from_value(claim.payload.clone()).map_err(|_| HandlerFailure {
                code: "invalid_job_payload",
                retryable: false,
            })?;
        let mail = self
            .prepare_mail_delivery(payload.delivery_id)
            .await
            .map_err(|_| HandlerFailure {
                code: "mail_preparation_failed",
                retryable: true,
            })?;
        let Some(mail) = mail else {
            return Ok(());
        };
        mailer
            .send(payload.delivery_id, &mail)
            .await
            .map_err(|error| HandlerFailure {
                code: "smtp_send_failed",
                retryable: error.is_retryable(),
            })?;
        let transaction = self.database.begin().await.map_err(|_| HandlerFailure {
            code: "delivery_storage_failed",
            retryable: true,
        })?;
        let now = database_time(&transaction)
            .await
            .map_err(|_| HandlerFailure {
                code: "delivery_storage_failed",
                retryable: true,
            })?;
        deliveries::finish_delivery(&transaction, payload.delivery_id, Some(now))
            .await
            .map_err(|_| HandlerFailure {
                code: "delivery_storage_failed",
                retryable: true,
            })?;
        transaction.commit().await.map_err(|_| HandlerFailure {
            code: "delivery_storage_failed",
            retryable: true,
        })?;
        Ok(())
    }

    async fn prepare_mail_delivery(
        &self,
        delivery_id: Uuid,
    ) -> ApplicationResult<Option<AuthenticationMail>> {
        let transaction = self.database.begin().await?;
        let Some(delivery) = deliveries::find_delivery(&transaction, delivery_id).await? else {
            return Ok(None);
        };
        if delivery.state != MailDeliveryState::Pending {
            return Ok(None);
        }
        let account = accounts::find_by_id(&transaction, delivery.account_id).await?;
        let ciphertext = delivery.ciphertext.as_deref().ok_or_else(|| {
            ApplicationError::internal(anyhow::anyhow!("pending mail payload missing"))
        })?;
        if delivery.expires_at <= database_time(&transaction).await? || account.is_none() {
            deliveries::finish_delivery(&transaction, delivery_id, None).await?;
            transaction.commit().await?;
            return Ok(None);
        }
        let plaintext = self
            .settings
            .secret_keys
            .decrypt(&delivery_context(delivery_id), ciphertext)?;
        let mail: AuthenticationMail =
            serde_json::from_slice(&plaintext).map_err(ApplicationError::internal)?;
        let valid = if let Some(purpose) = challenge_purpose(mail.purpose) {
            let hash = delivery.challenge_hash.as_deref().ok_or_else(|| {
                ApplicationError::internal(anyhow::anyhow!("action mail challenge missing"))
            })?;
            challenges::find_valid(&transaction, hash, purpose)
                .await?
                .is_some_and(|challenge| {
                    account.as_ref().is_some_and(|account| {
                        account.active && challenge.security_version == account.security_version
                    })
                })
        } else {
            delivery.challenge_hash.is_none()
                && account.as_ref().is_some_and(|account| {
                    account.verified
                        && (account.active || matches!(mail.purpose, MailPurpose::PasswordChanged))
                })
        };
        if !valid {
            deliveries::finish_delivery(&transaction, delivery_id, None).await?;
            transaction.commit().await?;
            return Ok(None);
        }
        transaction.commit().await?;
        Ok(Some(mail))
    }
}

fn delivery_context(delivery_id: Uuid) -> String {
    format!("authentication mail v1:{delivery_id}")
}
fn challenge_purpose(purpose: MailPurpose) -> Option<ChallengePurpose> {
    match purpose {
        MailPurpose::VerifyEmail => Some(ChallengePurpose::VerifyEmail),
        MailPurpose::ResetPassword => Some(ChallengePurpose::ResetPassword),
        MailPurpose::Welcome | MailPurpose::PasswordChanged => None,
    }
}
