//! Best-effort authentication audit recording (SEC-009).
//!
//! Recording must never break the auth flow it observes, so failures are logged and swallowed.

use crate::repository::auth_events::repository_create_auth_event;
use sea_orm::DatabaseConnection;
use sea_orm::prelude::IpNetwork;
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Parse an optional string IP into an `IpNetwork` (a bare address becomes a /32 or /128 host).
pub fn parse_ip(ip: Option<&str>) -> Option<IpNetwork> {
    ip.and_then(|s| s.parse::<IpNetwork>().ok())
}

/// Record an authentication audit event, best-effort: a write failure is logged, never propagated.
pub async fn record_auth_event(
    db: &DatabaseConnection,
    user_id: Option<Uuid>,
    event_type: &str,
    ip: Option<IpNetwork>,
    user_agent: Option<String>,
    metadata: Option<JsonValue>,
) {
    if let Err(e) =
        repository_create_auth_event(db, user_id, event_type, ip, user_agent, metadata).await
    {
        tracing::warn!(error = ?e, event_type, "Failed to record auth event");
    }
}
