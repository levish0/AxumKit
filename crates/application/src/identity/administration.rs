use super::{
    IdentityService,
    repository::{accounts, security},
};
use crate::{ApplicationError, ApplicationResult};
use jobs::TraceContext;
use sea_orm::TransactionTrait;
use uuid::Uuid;

impl IdentityService {
    /// Operator-only operation; callers must enforce access outside the public API.
    pub async fn set_administrator_by_operator(
        &self,
        account_id: Uuid,
        administrator: bool,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.database.begin().await?;
        let account = accounts::lock_account(&transaction, account_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        if administrator && (!account.active || !account.verified || account.totp_secret.is_none())
        {
            return Err(ApplicationError::InvalidInput(
                "administrator must be active, verified, and enrolled in MFA",
            ));
        }
        if account.administrator != administrator {
            accounts::set_administrator(&transaction, account, administrator).await?;
            self.revoke_account_credentials(&transaction, account_id)
                .await?;
            security::append_audit(
                &transaction,
                Some(account_id),
                if administrator {
                    "administrator_granted"
                } else {
                    "administrator_removed"
                },
                trace.request_id,
            )
            .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
