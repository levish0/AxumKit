use std::net::IpAddr;

/// Canonicalize an address so an IPv4 client is always represented as IPv4, even
/// when it arrives as an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) through a
/// dual-stack socket. Keeping a single canonical form across lookups and
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
}
