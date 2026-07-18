use crate::utils::crypto::constant_time::constant_time_str_eq;
use crate::utils::ip::canonicalize_ip;
use axum::http::HeaderMap;
use config::ServerConfig;
use std::net::{IpAddr, SocketAddr};

/// Extract real client IP address.
///
/// Priority order:
/// 1. `X-Real-Client-IP` — only when the request authenticates as the trusted SSR/BFF
///    proxy via a matching `X-Internal-Secret`. SSR-proxied requests reach us as
///    browser → CF → SSR → CF → backend, where the second CF hop rewrites
///    CF-Connecting-IP to the proxy's egress IP; the proxy forwards the true client IP
///    here instead.
/// 2. `CF-Connecting-IP` (valid IP only) — browser-direct requests through Cloudflare.
/// 3. ConnectInfo (direct connection socket address).
pub fn extract_ip_address(headers: &HeaderMap, addr: SocketAddr) -> String {
    let configured_secret = ServerConfig::get().internal_proxy_secret.as_deref();
    if let Some(ip) = trusted_proxy_client_ip(headers, configured_secret) {
        return canonicalize_ip(ip).to_string();
    }

    let resolved = headers
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or_else(|| addr.ip());
    canonicalize_ip(resolved).to_string()
}

/// Returns the proxy-forwarded client IP only when `X-Internal-Secret` matches the
/// configured shared secret. When no secret is configured the header is ignored, so a
/// public client can never spoof its IP via `X-Real-Client-IP`.
fn trusted_proxy_client_ip(headers: &HeaderMap, expected_secret: Option<&str>) -> Option<IpAddr> {
    let expected = expected_secret?;

    let provided = headers
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())?;
    // Constant-time compare: a plain `!=` here is a timing oracle for recovering the shared
    // secret, which would let a public client spoof `X-Real-Client-IP` (evade IP bans / rate limits).
    if !constant_time_str_eq(provided, expected) {
        return None;
    }

    headers
        .get("X-Real-Client-IP")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn socket_addr() -> SocketAddr {
        "127.0.0.1:8080".parse().unwrap()
    }

    #[test]
    fn uses_valid_cf_connecting_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("CF-Connecting-IP", HeaderValue::from_static("203.0.113.10"));

        assert_eq!(extract_ip_address(&headers, socket_addr()), "203.0.113.10");
    }

    #[test]
    fn trims_cf_connecting_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "CF-Connecting-IP",
            HeaderValue::from_static(" 2001:db8::1 "),
        );

        assert_eq!(extract_ip_address(&headers, socket_addr()), "2001:db8::1");
    }

    #[test]
    fn falls_back_to_socket_addr_when_cf_connecting_ip_is_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("CF-Connecting-IP", HeaderValue::from_static("not-an-ip"));

        assert_eq!(extract_ip_address(&headers, socket_addr()), "127.0.0.1");
    }

    #[test]
    fn ignores_other_proxy_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-IP", HeaderValue::from_static("203.0.113.20"));
        headers.insert(
            "X-Forwarded-For",
            HeaderValue::from_static("203.0.113.30, 198.51.100.1"),
        );

        assert_eq!(extract_ip_address(&headers, socket_addr()), "127.0.0.1");
    }

    #[test]
    fn trusts_real_client_ip_with_matching_secret() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Internal-Secret", HeaderValue::from_static("s3cret"));
        headers.insert("X-Real-Client-IP", HeaderValue::from_static("198.51.100.7"));
        // Proxy's own IP arrives in CF-Connecting-IP and must be overridden.
        headers.insert("CF-Connecting-IP", HeaderValue::from_static("10.0.0.1"));

        assert_eq!(
            trusted_proxy_client_ip(&headers, Some("s3cret")),
            Some("198.51.100.7".parse().unwrap())
        );
    }

    #[test]
    fn rejects_real_client_ip_with_wrong_secret() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Internal-Secret", HeaderValue::from_static("wrong"));
        headers.insert("X-Real-Client-IP", HeaderValue::from_static("198.51.100.7"));

        assert_eq!(trusted_proxy_client_ip(&headers, Some("s3cret")), None);
    }

    #[test]
    fn ignores_real_client_ip_when_no_secret_configured() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Internal-Secret", HeaderValue::from_static("s3cret"));
        headers.insert("X-Real-Client-IP", HeaderValue::from_static("198.51.100.7"));

        // No configured secret -> header is untrusted and ignored (anti-spoofing).
        assert_eq!(trusted_proxy_client_ip(&headers, None), None);
    }

    #[test]
    fn ignores_real_client_ip_without_secret_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Real-Client-IP", HeaderValue::from_static("198.51.100.7"));

        assert_eq!(trusted_proxy_client_ip(&headers, Some("s3cret")), None);
    }
}
