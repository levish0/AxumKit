use super::{
    IdentityService,
    entities::{account, challenge},
    repository::{accounts, challenges, security},
    validation::normalize_email,
};
use crate::{
    ApplicationError, ApplicationResult,
    identity::{ChallengePurpose, authentication_operation::AuthenticationOperation},
};
use auth_core::token::hash_token;
use jobs::TraceContext;
use sea_orm::{DatabaseTransaction, TransactionTrait};

impl IdentityService {
    pub async fn request_verification(
        &self,
        email: &str,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.request_authentication_mail(email, mail::MailPurpose::VerifyEmail, trace)
            .await
    }

    pub async fn request_password_reset(
        &self,
        email: &str,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.request_authentication_mail(email, mail::MailPurpose::ResetPassword, trace)
            .await
    }

    async fn request_authentication_mail(
        &self,
        email: &str,
        purpose: mail::MailPurpose,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let email = normalize_email(email)?;
        self.limit_authentication_attempts(AuthenticationOperation::AuthenticationMail, &email)
            .await?;
        let Some(snapshot) = accounts::find_by_email(&self.database, &email).await? else {
            return Ok(());
        };
        let transaction = self.database.begin().await?;
        let Some(account) = accounts::lock_account(&transaction, snapshot.id).await? else {
            return Ok(());
        };
        let eligible = account.active
            && match purpose {
                mail::MailPurpose::VerifyEmail => !account.verified,
                mail::MailPurpose::ResetPassword => {
                    account.verified && account.password_hash.is_some()
                }
                _ => {
                    return Err(ApplicationError::InvalidInput(
                        "invalid action mail purpose",
                    ));
                }
            };
        if eligible {
            self.register_authentication_mail(&transaction, &account, purpose, trace)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }

    pub async fn verify_email(&self, token: &str, trace: TraceContext) -> ApplicationResult<()> {
        self.limit_authentication_attempts(
            AuthenticationOperation::VerifyEmail,
            &hash_token(token),
        )
        .await?;
        let transaction = self.database.begin().await?;
        let (account, challenge) = self
            .lock_challenge_account(&transaction, token, ChallengePurpose::VerifyEmail)
            .await?;
        if account.verified {
            return Err(ApplicationError::Unauthenticated);
        }
        if !challenges::delete_challenge(&transaction, challenge.id).await? {
            return Err(ApplicationError::Unauthenticated);
        }
        let account = accounts::mark_verified(&transaction, account).await?;
        self.register_account_notification(
            &transaction,
            &account,
            mail::MailPurpose::Welcome,
            trace,
        )
        .await?;
        security::append_audit(
            &transaction,
            Some(account.id),
            "email_verified",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn reset_password(
        &self,
        token: &str,
        new_password: String,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.limit_authentication_attempts(
            AuthenticationOperation::ResetPassword,
            &hash_token(token),
        )
        .await?;
        let password_hash = self.hash_new_password(new_password).await?;
        let transaction = self.database.begin().await?;
        let (account, challenge) = self
            .lock_challenge_account(&transaction, token, ChallengePurpose::ResetPassword)
            .await?;
        if !account.verified || account.password_hash.is_none() {
            return Err(ApplicationError::Unauthenticated);
        }
        if !challenges::delete_challenge(&transaction, challenge.id).await? {
            return Err(ApplicationError::Unauthenticated);
        }
        let account = accounts::replace_password(&transaction, account, password_hash).await?;
        self.revoke_account_credentials(&transaction, account.id)
            .await?;
        self.register_account_notification(
            &transaction,
            &account,
            mail::MailPurpose::PasswordChanged,
            trace,
        )
        .await?;
        security::append_audit(
            &transaction,
            Some(account.id),
            "password_reset",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn lock_challenge_account(
        &self,
        transaction: &DatabaseTransaction,
        token: &str,
        purpose: ChallengePurpose,
    ) -> ApplicationResult<(account::Model, challenge::Model)> {
        if token.len() != 43 {
            return Err(ApplicationError::Unauthenticated);
        }
        let token_hash = hash_token(token);
        let candidate = challenges::find_valid(transaction, &token_hash, purpose)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        let account = accounts::lock_account(transaction, candidate.account_id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        let challenge = challenges::find_valid(transaction, &token_hash, purpose)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if !account.active || challenge.security_version != account.security_version {
            return Err(ApplicationError::Unauthenticated);
        }
        Ok((account, challenge))
    }
}
