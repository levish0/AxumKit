use crate::service::auth::session_types::SessionContext;
use errors::errors::Errors;
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
    let ip = parse_attribution_ip(ip_address)?;

    match session {
        Some(s) => Ok((Some(s.user_id), Some(ip))),
        None => Ok((None, Some(ip))),
    }
}

/// Parses the caller's canonicalized IP for audit/attribution purposes.
///
/// The string arrives already canonicalized from the `extract_ip_address`
/// boundary, so a parse failure is an upstream bug. Rather than silently
/// recording NULL and losing the audit trail (multi-account detection, ban
/// evasion), the request fails with `Errors::InvalidIpAddress` — the same
/// policy as `extract_user_or_ip`.
pub fn parse_attribution_ip(ip_address: &str) -> Result<IpNetwork, Errors> {
    ip_address
        .parse::<IpNetwork>()
        .map_err(|_| Errors::InvalidIpAddress)
}
