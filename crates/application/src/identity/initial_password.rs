use super::{
    AuthenticatedSession, IdentityService,
    repository::{accounts, security},
    validation::validate_new_password,
};
use crate::{
    ApplicationError, ApplicationResult,
    identity::authentication_operation::AuthenticationOperation,
};
use jobs::TraceContext;
use sea_orm::TransactionTrait;

impl IdentityService {
    #[tracing::instrument(skip_all, fields(request_id = %trace.request_id))]
    pub async fn set_initial_password(
        &self,
        actor: &AuthenticatedSession,
        new_password: String,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        self.limit_authentication_attempts(
            AuthenticationOperation::SetInitialPassword,
            &actor.account_id().to_string(),
        )
        .await?;
        validate_new_password(&new_password)?;
        let snapshot = accounts::find_by_id(&self.database, actor.account_id())
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if snapshot.password_hash.is_some() {
            return Err(ApplicationError::Conflict);
        }
        let password_hash = self.hash_new_password(new_password).await?;
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        // Another request can add a password after the unlocked snapshot.
        if account.password_hash.is_some() {
            return Err(ApplicationError::Conflict);
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
            "initial_password_set",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
