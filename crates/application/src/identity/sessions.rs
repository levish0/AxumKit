use super::{
    AccountSummary, AuthenticatedSession, IdentityService, SessionCredentials, SessionSummary,
    repository::{accounts, security, sessions},
    service::{principal, require_authenticatable_account},
};
use crate::{
    ApplicationError, ApplicationResult,
    database::database_time,
    pagination::{Page, PageRequest},
};
use auth_core::token::{generate_secure_token, hash_token};
use chrono::Duration;
use jobs::TraceContext;
use sea_orm::TransactionTrait;
use uuid::Uuid;

impl IdentityService {
    #[tracing::instrument(skip_all)]
    pub async fn authenticate_session(
        &self,
        token: &str,
    ) -> ApplicationResult<AuthenticatedSession> {
        if token.len() != 43 {
            return Err(ApplicationError::Unauthenticated);
        }
        let token_hash = hash_token(token);
        let snapshot = sessions::find_by_hash(&self.database, &token_hash)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        let transaction = self.database.begin().await?;
        let account = accounts::lock_account(&transaction, snapshot.account_id)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        require_authenticatable_account(&account)?;
        let session = sessions::find_by_hash(&transaction, &token_hash)
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if !account.active
            || !account.verified
            || session.security_version != account.security_version
        {
            return Err(ApplicationError::Unauthenticated);
        }
        let now = database_time(&transaction).await?;
        if now - session.last_seen_at >= Duration::minutes(1) {
            sessions::touch_session(
                &transaction,
                session.id,
                now,
                (now + Duration::seconds(self.settings.session_idle_seconds))
                    .min(session.expires_at),
            )
            .await?;
        }
        let actor = AuthenticatedSession {
            principal: principal(&account),
            session_id: session.id,
            security_version: account.security_version,
            token_hash,
        };
        transaction.commit().await?;
        Ok(actor)
    }

    pub async fn get_account(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<AccountSummary> {
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        transaction.commit().await?;
        Ok(account.into())
    }

    pub async fn list_sessions(
        &self,
        actor: &AuthenticatedSession,
        limit: Option<u64>,
        cursor: Option<&str>,
    ) -> ApplicationResult<Page<SessionSummary>> {
        let scope = format!("sessions:{}", actor.account_id());
        let page = PageRequest::parse(limit, cursor, &scope)?;
        let transaction = self.database.begin().await?;
        self.lock_authenticated_account(&transaction, actor, false)
            .await?;
        let sessions = sessions::list_for_account(&transaction, actor.account_id(), page).await?;
        transaction.commit().await?;
        Ok(Page::from_rows(
            sessions.into_iter().map(SessionSummary::from).collect(),
            page,
            &scope,
            |session| session.id,
        ))
    }

    #[tracing::instrument(skip_all, fields(account_id = %actor.account_id(), session_id = %actor.session_id()))]
    pub async fn rotate_session(
        &self,
        actor: &AuthenticatedSession,
        trace: TraceContext,
    ) -> ApplicationResult<SessionCredentials> {
        let transaction = self.database.begin().await?;
        self.lock_authenticated_account(&transaction, actor, false)
            .await?;
        let session = sessions::find_by_id(&transaction, actor.session_id())
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        let token = generate_secure_token();
        sessions::rotate_session(&transaction, session.id, hash_token(&token)).await?;
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "session_rotated",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(SessionCredentials {
            session_id: session.id,
            token,
            expires_at: session.expires_at,
        })
    }

    pub async fn revoke_session(
        &self,
        actor: &AuthenticatedSession,
        session_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.database.begin().await?;
        self.lock_authenticated_account(&transaction, actor, false)
            .await?;
        if !sessions::revoke_session(&transaction, actor.account_id(), session_id).await? {
            return Err(ApplicationError::NotFound);
        }
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "session_revoked",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn revoke_all_sessions(
        &self,
        actor: &AuthenticatedSession,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.database.begin().await?;
        self.lock_authenticated_account(&transaction, actor, true)
            .await?;
        sessions::revoke_all(&transaction, actor.account_id()).await?;
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "all_sessions_revoked",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn deactivate_account(
        &self,
        actor: &AuthenticatedSession,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.database.begin().await?;
        let account = self
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        accounts::set_active(&transaction, account, false).await?;
        self.revoke_account_credentials(&transaction, actor.account_id())
            .await?;
        security::append_audit(
            &transaction,
            Some(actor.account_id()),
            "account_deactivated",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}
