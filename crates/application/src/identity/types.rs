use crate::authorization::Principal;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthenticatedSession {
    pub(crate) principal: Principal,
    pub(crate) session_id: Uuid,
    pub(crate) security_version: i64,
    pub(crate) token_hash: String,
}
impl AuthenticatedSession {
    pub fn account_id(&self) -> Uuid {
        self.principal.account_id
    }
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

pub struct SessionCredentials {
    pub session_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}
pub enum LoginOutcome {
    Session(SessionCredentials),
    MfaRequired { challenge: String },
}

pub struct AccountSummary {
    pub id: Uuid,
    pub email: String,
    pub verified: bool,
    pub mfa_enabled: bool,
    pub has_password: bool,
}
impl From<super::entities::account::Model> for AccountSummary {
    fn from(account: super::entities::account::Model) -> Self {
        Self {
            id: account.id,
            email: account.email,
            verified: account.verified,
            mfa_enabled: account.totp_secret.is_some(),
            has_password: account.password_hash.is_some(),
        }
    }
}
pub struct SessionSummary {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
impl From<super::entities::session::Model> for SessionSummary {
    fn from(session: super::entities::session::Model) -> Self {
        Self {
            id: session.id,
            created_at: session.created_at,
            last_seen_at: session.last_seen_at,
            expires_at: session.expires_at,
        }
    }
}
