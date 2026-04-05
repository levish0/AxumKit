use crate::service::auth::session_types::SessionContext;
use axumkit_errors::errors::Errors;
use sea_orm::prelude::IpNetwork;
use uuid::Uuid;

/// Extract user_id and IP from SessionContext
///
/// # Returns
/// - Logged-in user: `(Some(user_id), Some(ip_network))`
/// - Anonymous user: `(None, Some(ip_network))`
///
/// IP is always recorded (for multi-account detection, ban evasion tracking)
///
/// # Errors
/// - `Errors::InvalidIpAddress` - When IP address parsing fails
pub fn extract_user_or_ip(
    session: Option<&SessionContext>,
    ip_address: &str,
) -> Result<(Option<Uuid>, Option<IpNetwork>), Errors> {
    let ip = ip_address
        .parse::<IpNetwork>()
        .map_err(|_| Errors::InvalidIpAddress)?;

    match session {
        Some(s) => Ok((Some(s.user_id), Some(ip))),
        None => Ok((None, Some(ip))),
    }
}
