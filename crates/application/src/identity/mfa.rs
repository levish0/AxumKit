use super::{
    AuthenticatedSession, IdentityService, SessionCredentials,
    entities::account,
    repository::{accounts, challenges, security},
};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    identity::{ChallengePurpose, authentication_operation::AuthenticationOperation},
};
use auth_core::{
    constant_time::constant_time_str_eq,
    token::{generate_secure_token, hash_token},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Duration;
use jobs::TraceContext;
use sea_orm::{DatabaseTransaction, TransactionTrait};
use totp_rs::{Algorithm, Builder, Totp};

pub struct TotpEnrollment {
    pub challenge: String,
    pub authenticator_url: String,
}
pub enum MfaProof {
    Totp(String),
    RecoveryCode(String),
}

impl IdentityService {
    pub async fn begin_totp_enrollment(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<TotpEnrollment> {
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        if account.totp_secret.is_some() {
            return Err(ApplicationError::Conflict);
        }
        let secret = URL_SAFE_NO_PAD
            .decode(generate_secure_token())
            .map_err(ApplicationError::internal)?;
        let totp = self.totp(&account, secret.clone())?;
        let encrypted_secret = self
            .settings
            .secret_keys
            .encrypt(&totp_context(account.id), &secret)?;
        let challenge = generate_secure_token();
        challenges::delete_for_account(&transaction, account.id, Some(ChallengePurpose::TotpSetup))
            .await?;
        challenges::insert_challenge(
            &transaction,
            account.id,
            hash_token(&challenge),
            ChallengePurpose::TotpSetup,
            account.security_version,
            database_time(&transaction).await? + Duration::minutes(5),
            Some(encrypted_secret),
        )
        .await?;
        transaction.commit().await?;
        Ok(TotpEnrollment {
            challenge,
            authenticator_url: totp.to_url().map_err(ApplicationError::internal)?,
        })
    }

    pub async fn confirm_totp_enrollment(
        &self,
        actor: &AuthenticatedSession,
        token: &str,
        code: &str,
        trace: TraceContext,
    ) -> ApplicationResult<Vec<String>> {
        self.limit_authentication_attempts(
            AuthenticationOperation::TotpSetup,
            &actor.account_id().to_string(),
        )
        .await?;
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        let challenge = challenges::find_valid(
            &transaction,
            &hash_token(token),
            ChallengePurpose::TotpSetup,
        )
        .await?
        .ok_or(ApplicationError::Unauthenticated)?;
        if challenge.account_id != account.id
            || challenge.security_version != account.security_version
            || account.totp_secret.is_some()
        {
            return Err(ApplicationError::Unauthenticated);
        }
        let encrypted_secret = challenge.secret.ok_or_else(|| {
            ApplicationError::internal(anyhow::anyhow!("enrollment secret missing"))
        })?;
        let secret = self
            .settings
            .secret_keys
            .decrypt(&totp_context(account.id), &encrypted_secret)?;
        let step = matching_totp_step(
            &self.totp(&account, secret)?,
            code,
            database_time(&transaction).await?.timestamp(),
            -1,
        )?;
        if !challenges::delete_challenge(&transaction, challenge.id).await? {
            return Err(ApplicationError::Unauthenticated);
        }
        accounts::set_totp(&transaction, account, Some(encrypted_secret), step).await?;
        let recovery_codes = self
            .create_recovery_codes(&transaction, actor.account_id())
            .await?;
        self.revoke_account_credentials(&transaction, actor.account_id())
            .await?;
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "totp_enabled",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(recovery_codes)
    }

    pub async fn complete_mfa_login(
        &self,
        token: &str,
        proof: MfaProof,
        trace: TraceContext,
    ) -> ApplicationResult<SessionCredentials> {
        let candidate =
            challenges::find_valid(&self.database, &hash_token(token), ChallengePurpose::Mfa)
                .await?;
        let limit_identity = candidate.map_or_else(
            || hash_token(token),
            |challenge| challenge.account_id.to_string(),
        );
        self.limit_authentication_attempts(AuthenticationOperation::Mfa, &limit_identity)
            .await?;
        let transaction = self.database.begin().await?;
        let (account, challenge) = self
            .lock_challenge_account(&transaction, token, ChallengePurpose::Mfa)
            .await?;
        if !account.verified {
            return Err(ApplicationError::Unauthenticated);
        }
        self.consume_mfa_proof(&transaction, &account, proof)
            .await?;
        if !challenges::delete_challenge(&transaction, challenge.id).await? {
            return Err(ApplicationError::Unauthenticated);
        }
        let credentials = self.issue_session(&transaction, &account, trace).await?;
        transaction.commit().await?;
        Ok(credentials)
    }

    pub async fn disable_totp(
        &self,
        actor: &AuthenticatedSession,
        proof: MfaProof,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.limit_authentication_attempts(
            AuthenticationOperation::Mfa,
            &actor.account_id().to_string(),
        )
        .await?;
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        if account.administrator {
            return Err(ApplicationError::Forbidden);
        }
        self.consume_mfa_proof(&transaction, &account, proof)
            .await?;
        accounts::set_totp(&transaction, account, None, -1).await?;
        security::replace_recovery_codes(&transaction, actor.account_id(), Vec::new()).await?;
        self.revoke_account_credentials(&transaction, actor.account_id())
            .await?;
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "totp_disabled",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn regenerate_recovery_codes(
        &self,
        actor: &AuthenticatedSession,
        proof: MfaProof,
        trace: TraceContext,
    ) -> ApplicationResult<Vec<String>> {
        self.limit_authentication_attempts(
            AuthenticationOperation::Mfa,
            &actor.account_id().to_string(),
        )
        .await?;
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        self.consume_mfa_proof(&transaction, &account, proof)
            .await?;
        let recovery_codes = self.create_recovery_codes(&transaction, account.id).await?;
        accounts::advance_security_version(&transaction, account.id).await?;
        self.revoke_account_credentials(&transaction, account.id)
            .await?;
        security::append_audit(
            &transaction,
            Some(account.id),
            "recovery_codes_regenerated",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(recovery_codes)
    }

    async fn consume_mfa_proof(
        &self,
        transaction: &DatabaseTransaction,
        account: &account::Model,
        proof: MfaProof,
    ) -> ApplicationResult<()> {
        let encrypted_secret = account
            .totp_secret
            .as_deref()
            .ok_or(ApplicationError::Unauthenticated)?;
        match proof {
            MfaProof::Totp(code) => {
                let secret = self
                    .settings
                    .secret_keys
                    .decrypt(&totp_context(account.id), encrypted_secret)?;
                let step = matching_totp_step(
                    &self.totp(account, secret)?,
                    &code,
                    database_time(transaction).await?.timestamp(),
                    account.totp_last_step,
                )?;
                accounts::record_totp_step(transaction, account.clone(), step).await?;
            }
            MfaProof::RecoveryCode(code) => {
                if !security::consume_recovery_code(transaction, account.id, &hash_token(&code))
                    .await?
                {
                    return Err(ApplicationError::Unauthenticated);
                }
            }
        }
        Ok(())
    }

    async fn create_recovery_codes(
        &self,
        transaction: &DatabaseTransaction,
        account_id: uuid::Uuid,
    ) -> ApplicationResult<Vec<String>> {
        let codes: Vec<_> = (0..10).map(|_| generate_secure_token()).collect();
        security::replace_recovery_codes(
            transaction,
            account_id,
            codes.iter().map(|code| hash_token(code)).collect(),
        )
        .await?;
        Ok(codes)
    }

    fn totp(&self, account: &account::Model, secret: Vec<u8>) -> ApplicationResult<Totp> {
        Builder::new()
            .with_algorithm(Algorithm::SHA1)
            .with_digits(6)
            .with_skew(1)
            .with_step_duration(30)
            .with_secret(secret)
            .with_issuer(Some(self.settings.application_name.replace(':', " ")))
            .with_account_name(account.email.clone())
            .build()
            .map_err(ApplicationError::internal)
    }
}

fn totp_context(account_id: uuid::Uuid) -> String {
    format!("totp secret v1:{account_id}")
}

fn matching_totp_step(
    totp: &Totp,
    code: &str,
    now_seconds: i64,
    last_step: i64,
) -> ApplicationResult<i64> {
    if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApplicationError::Unauthenticated);
    }
    let current_step = now_seconds / 30;
    for step in (current_step - 1..=current_step + 1).rev() {
        if step > last_step
            && step >= 0
            && constant_time_str_eq(&totp.generate(step as u64 * 30).to_string(), code)
        {
            return Ok(step);
        }
    }
    Err(ApplicationError::Unauthenticated)
}
