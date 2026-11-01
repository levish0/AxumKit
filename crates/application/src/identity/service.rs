use super::{
    AuthenticatedSession, LoginOutcome, SessionCredentials,
    entities::account,
    password_operations::PasswordOperations,
    repository::{accounts, challenges, security, sessions},
    validation::{normalize_email, validate_new_password},
};
use crate::{
    ApplicationError, ApplicationResult,
    authorization::{Action, Authorization, Principal},
    config::IdentitySettings,
    database::database_time,
    identity::{ChallengePurpose, authentication_operation::AuthenticationOperation},
};
use auth_core::{
    keyed_hash,
    token::{generate_secure_token, hash_token},
};
use chrono::Duration;
use jobs::TraceContext;
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use std::sync::Arc;

#[derive(Clone)]
pub struct IdentityService {
    pub(crate) database: DatabaseConnection,
    pub(crate) settings: IdentitySettings,
    pub(crate) authorization: Arc<Authorization>,
    passwords: PasswordOperations,
}

impl IdentityService {
    pub async fn new(
        database: DatabaseConnection,
        settings: IdentitySettings,
        authorization: Arc<Authorization>,
    ) -> ApplicationResult<Self> {
        settings.validate()?;
        let passwords = PasswordOperations::new(settings.password_concurrency).await?;
        Ok(Self {
            database,
            settings,
            authorization,
            passwords,
        })
    }

    #[tracing::instrument(skip_all, fields(request_id = %trace.request_id))]
    pub async fn register_account(
        &self,
        email: &str,
        password: String,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let email = normalize_email(email)?;
        self.limit_authentication_attempts(AuthenticationOperation::Signup, &email)
            .await?;
        validate_new_password(&password)?;
        let password_hash = self.passwords.hash(password).await?;
        let transaction = self.database.begin().await?;
        if accounts::find_by_email(&transaction, &email)
            .await?
            .is_some()
        {
            return Ok(());
        }
        let account =
            match accounts::insert_account(&transaction, email, Some(password_hash), false).await {
                Ok(account) => account,
                Err(error)
                    if matches!(
                        error.sql_err(),
                        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
                    ) =>
                {
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
            };
        self.register_authentication_mail(
            &transaction,
            &account,
            mail::MailPurpose::VerifyEmail,
            trace,
        )
        .await?;
        security::append_audit(
            &transaction,
            Some(account.id),
            "account_registered",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(request_id = %trace.request_id))]
    pub async fn login_with_password(
        &self,
        email: &str,
        password: String,
        trace: TraceContext,
    ) -> ApplicationResult<LoginOutcome> {
        let email = normalize_email(email)?;
        self.limit_authentication_attempts(AuthenticationOperation::Login, &email)
            .await?;
        let snapshot = accounts::find_by_email(&self.database, &email).await?;
        let password_matches = self
            .passwords
            .verify(
                password,
                snapshot
                    .as_ref()
                    .and_then(|account| account.password_hash.as_deref()),
            )
            .await?;
        let snapshot = snapshot
            .filter(|_| password_matches)
            .ok_or(ApplicationError::Unauthenticated)?;
        let transaction = self.database.begin().await?;
        let account = accounts::lock_account(&transaction, snapshot.id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if account.security_version != snapshot.security_version
            || account.password_hash != snapshot.password_hash
        {
            return Err(ApplicationError::Unauthenticated);
        }
        let outcome = self
            .complete_primary_authentication(&transaction, &account, trace)
            .await?;
        transaction.commit().await?;
        Ok(outcome)
    }

    pub(crate) async fn complete_primary_authentication(
        &self,
        transaction: &DatabaseTransaction,
        account: &account::Model,
        trace: TraceContext,
    ) -> ApplicationResult<LoginOutcome> {
        require_authenticatable_account(account)?;
        if account.totp_secret.is_some() {
            let challenge = generate_secure_token();
            challenges::insert_challenge(
                transaction,
                account.id,
                hash_token(&challenge),
                ChallengePurpose::Mfa,
                account.security_version,
                database_time(transaction).await? + Duration::minutes(5),
                None,
            )
            .await?;
            Ok(LoginOutcome::MfaRequired { challenge })
        } else {
            Ok(LoginOutcome::Session(
                self.issue_session(transaction, account, trace).await?,
            ))
        }
    }

    pub(crate) async fn issue_session(
        &self,
        transaction: &DatabaseTransaction,
        account: &account::Model,
        trace: TraceContext,
    ) -> ApplicationResult<SessionCredentials> {
        require_authenticatable_account(account)?;
        let now = database_time(transaction).await?;
        let token = generate_secure_token();
        let expires_at = now + Duration::seconds(self.settings.session_absolute_seconds);
        let session = sessions::insert_session(
            transaction,
            account.id,
            hash_token(&token),
            account.security_version,
            expires_at,
            now + Duration::seconds(self.settings.session_idle_seconds),
        )
        .await?;
        security::append_audit(
            transaction,
            Some(account.id),
            "session_issued",
            trace.request_id,
        )
        .await?;
        Ok(SessionCredentials {
            session_id: session.id,
            token,
            expires_at,
        })
    }

    pub(crate) async fn lock_authenticated_account(
        &self,
        transaction: &DatabaseTransaction,
        actor: &AuthenticatedSession,
        require_recent_authentication: bool,
    ) -> ApplicationResult<account::Model> {
        let account = accounts::lock_account(transaction, actor.account_id())
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        require_authenticatable_account(&account)?;
        let session = sessions::find_by_id(transaction, actor.session_id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if !account.active
            || !account.verified
            || account.security_version != actor.security_version
            || session.security_version != account.security_version
            || session.account_id != account.id
            || session.token_hash != actor.token_hash
        {
            return Err(ApplicationError::Unauthenticated);
        }
        if require_recent_authentication
            && database_time(transaction).await? - session.authenticated_at > Duration::minutes(5)
        {
            return Err(ApplicationError::Forbidden);
        }
        self.authorization.require(
            principal(&account),
            Action::ManageAccount,
            account.id,
            account.id,
            false,
        )?;
        Ok(account)
    }

    pub(super) async fn limit_authentication_attempts(
        &self,
        operation: AuthenticationOperation,
        identity: &str,
    ) -> ApplicationResult<()> {
        // These attempts must survive a later authentication transaction rollback.
        let transaction = self.database.begin().await?;
        let expires_at = database_time(&transaction).await? + Duration::minutes(1);
        let key = keyed_hash::hash_hex(
            self.settings.rate_limit_key.expose().as_bytes(),
            "authentication identity rate v1",
            format!("{}:{identity}", operation.as_str()).as_bytes(),
        );
        let attempts = security::consume_rate_limit(&transaction, key, expires_at).await?;
        transaction.commit().await?;
        if attempts > self.settings.authentication_identity_limit_per_minute {
            return Err(ApplicationError::RateLimited);
        }
        Ok(())
    }

    pub async fn limit_client_authentication(&self, client_address: &str) -> ApplicationResult<()> {
        let transaction = self.database.begin().await?;
        let expires_at = database_time(&transaction).await? + Duration::minutes(1);
        let key = keyed_hash::hash_hex(
            self.settings.rate_limit_key.expose().as_bytes(),
            "authentication client rate v1",
            client_address.as_bytes(),
        );
        let attempts = security::consume_rate_limit(&transaction, key, expires_at).await?;
        transaction.commit().await?;
        if attempts > self.settings.authentication_client_limit_per_minute {
            return Err(ApplicationError::RateLimited);
        }
        Ok(())
    }

    pub async fn change_password(
        &self,
        actor: &AuthenticatedSession,
        current_password: String,
        new_password: String,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.limit_authentication_attempts(
            AuthenticationOperation::ChangePassword,
            &actor.account_id().to_string(),
        )
        .await?;
        validate_new_password(&new_password)?;
        let snapshot = accounts::find_by_id(&self.database, actor.account_id())
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if !self
            .passwords
            .verify(current_password, snapshot.password_hash.as_deref())
            .await?
            || snapshot.password_hash.is_none()
        {
            return Err(ApplicationError::Unauthenticated);
        }
        let password_hash = self.passwords.hash(new_password).await?;
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        if account.password_hash != snapshot.password_hash {
            return Err(ApplicationError::Unauthenticated);
        }
        let account = accounts::replace_password(&transaction, account, password_hash).await?;
        self.revoke_account_credentials(&transaction, actor.account_id())
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
            Some(actor.account_id()),
            "password_changed",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub(crate) async fn hash_new_password(&self, password: String) -> ApplicationResult<String> {
        validate_new_password(&password)?;
        self.passwords.hash(password).await
    }

    pub(crate) async fn revoke_account_credentials(
        &self,
        transaction: &DatabaseTransaction,
        account_id: uuid::Uuid,
    ) -> ApplicationResult<()> {
        sessions::revoke_all(transaction, account_id).await?;
        challenges::delete_for_account(transaction, account_id, None).await?;
        Ok(())
    }
}

pub(crate) fn principal(account: &account::Model) -> Principal {
    Principal {
        account_id: account.id,
        active: account.active,
        verified: account.verified,
        administrator: account.administrator,
    }
}

pub(crate) fn require_authenticatable_account(account: &account::Model) -> ApplicationResult<()> {
    if !account.active
        || !account.verified
        || (account.administrator && account.totp_secret.is_none())
    {
        return Err(ApplicationError::Unauthenticated);
    }
    Ok(())
}
