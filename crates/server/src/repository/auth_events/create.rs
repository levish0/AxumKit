use entity::auth_events::{ActiveModel as AuthEventActiveModel, Model as AuthEventModel};
use errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Authentication event types recorded in `auth_events` (SEC-009 / OWASP ASVS V16).
///
/// A closed vocabulary kept as string constants (mirrors `constants::ActionLogAction`): the column
/// is `text`, so new event types are additive with no enum migration.
pub const AUTH_EVENT_LOGIN_SUCCESS: &str = "login_success";
pub const AUTH_EVENT_LOGIN_FAILED: &str = "login_failed";
pub const AUTH_EVENT_LOGOUT: &str = "logout";
pub const AUTH_EVENT_PASSWORD_CHANGED: &str = "password_changed";
pub const AUTH_EVENT_PASSWORD_RESET: &str = "password_reset";
pub const AUTH_EVENT_EMAIL_CHANGE_REQUESTED: &str = "email_change_requested";
pub const AUTH_EVENT_EMAIL_CHANGED: &str = "email_changed";
pub const AUTH_EVENT_TOTP_ENABLED: &str = "totp_enabled";
pub const AUTH_EVENT_TOTP_DISABLED: &str = "totp_disabled";
pub const AUTH_EVENT_NEW_DEVICE: &str = "new_device_login";

/// Insert one authentication audit event.
///
/// `user_id` is `None` for failed logins on an unknown email. Callers should treat recording as
/// best-effort (log on error) so an audit-write failure never breaks the authentication flow.
pub async fn repository_create_auth_event<C>(
    conn: &C,
    user_id: Option<Uuid>,
    event_type: &str,
    ip: Option<IpNetwork>,
    user_agent: Option<String>,
    metadata: Option<JsonValue>,
) -> Result<AuthEventModel, Errors>
where
    C: ConnectionTrait,
{
    let event = AuthEventActiveModel {
        id: Default::default(),
        user_id: Set(user_id),
        event_type: Set(event_type.to_string()),
        ip: Set(ip),
        user_agent: Set(user_agent),
        metadata: Set(metadata),
        created_at: Default::default(), // DB default now()
    };

    Ok(event.insert(conn).await?)
}
