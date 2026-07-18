use sea_orm::prelude::IpNetwork;
use std::net::IpAddr;

/// Canonicalize an address so an IPv4 client is always represented as IPv4, even
/// when it arrives as an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) through a
/// dual-stack socket. Keeping a single canonical form across IP bans, lookups, and
/// logging prevents the same client from being represented two different ways.
pub fn canonicalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(v6)),
        other => other,
    }
}

/// Canonicalize a single IPv4-mapped host network (`/128`) to its IPv4 form.
/// Genuine IPv6 ranges (prefix < 128) are left untouched.
pub fn canonicalize_ip_network(net: IpNetwork) -> IpNetwork {
    if net.prefix() == 128 {
        let canonical = canonicalize_ip(net.ip());
        if canonical != net.ip() {
            // `IpNetwork::from` yields a single-host network (/32 for IPv4).
            return IpNetwork::from(canonical);
        }
    }
    net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_ip_preserves_ipv4() {
        let ip: IpAddr = "203.0.113.5".parse().unwrap();
        assert_eq!(canonicalize_ip(ip), ip);
    }

    #[test]
    fn canonicalize_ip_unwraps_ipv4_mapped_ipv6() {
        let mapped: IpAddr = "::ffff:203.0.113.5".parse().unwrap();
        let expected: IpAddr = "203.0.113.5".parse().unwrap();
        assert_eq!(canonicalize_ip(mapped), expected);
    }

    #[test]
    fn canonicalize_ip_preserves_native_ipv6() {
        let ip: IpAddr = "2001:db8::1".parse().unwrap();
        assert_eq!(canonicalize_ip(ip), ip);
    }

    #[test]
    fn canonicalize_network_unwraps_mapped_host() {
        let mapped: IpNetwork = "::ffff:203.0.113.5".parse().unwrap();
        let expected: IpNetwork = "203.0.113.5".parse().unwrap();
        assert_eq!(canonicalize_ip_network(mapped), expected);
    }

    #[test]
    fn canonicalize_network_preserves_ipv4() {
        let net: IpNetwork = "203.0.113.0/24".parse().unwrap();
        assert_eq!(canonicalize_ip_network(net), net);
    }

    #[test]
    fn canonicalize_network_preserves_native_ipv6_range() {
        let net: IpNetwork = "2001:db8::/32".parse().unwrap();
        assert_eq!(canonicalize_ip_network(net), net);
    }
}
