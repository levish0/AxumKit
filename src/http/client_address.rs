use super::error::ApiError;
use application::config::SecretString;
use auth_core::constant_time::constant_time_str_eq;
use axum::http::HeaderMap;
use std::net::IpAddr;

/// Production ingress must pass through Cloudflare Tunnel. A matching proxy
/// secret permits an SSR server to preserve the original client IP.
pub(super) fn client_address(
    peer: Option<IpAddr>,
    headers: &HeaderMap,
    proxy_secret: Option<&SecretString>,
) -> Result<Option<IpAddr>, ApiError> {
    let trusted_proxy = proxy_secret.is_some_and(|expected| {
        let mut secrets = headers.get_all("x-internal-secret").iter();
        let provided = secrets.next().and_then(|value| value.to_str().ok());
        secrets.next().is_none()
            && provided.is_some_and(|provided| constant_time_str_eq(provided, expected.expose()))
    });
    if trusted_proxy && let Some(ip) = header_address(headers, "x-real-client-ip")? {
        return Ok(Some(ip));
    }
    Ok(header_address(headers, "cf-connecting-ip")?.or_else(|| peer.map(canonical_address)))
}

fn header_address(headers: &HeaderMap, name: &str) -> Result<Option<IpAddr>, ApiError> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err(ApiError::invalid_input());
    }
    value
        .to_str()
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .map(canonical_address)
        .map(Some)
        .ok_or_else(ApiError::invalid_input)
}

fn canonical_address(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(address) => address
            .to_ipv4_mapped()
            .map_or(IpAddr::V6(address), IpAddr::V4),
        address => address,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn secret() -> SecretString {
        SecretString::new(auth_core::token::generate_secure_token())
    }

    #[test]
    fn cloudflare_client_identity_survives_proxy_pod_replacement() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("192.0.2.10"));
        for peer in ["10.42.0.5", "10.42.1.9"] {
            assert_eq!(
                client_address(Some(peer.parse().unwrap()), &headers, None).unwrap(),
                Some("192.0.2.10".parse().unwrap())
            );
        }
    }

    #[test]
    fn only_a_matching_secret_can_override_cloudflare_identity() {
        let expected = secret();
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("192.0.2.10"));
        headers.insert("x-real-client-ip", HeaderValue::from_static("198.51.100.7"));
        for provided in [
            None,
            Some(HeaderValue::from_static("wrong")),
            Some(HeaderValue::from_bytes(&[0xff]).unwrap()),
        ] {
            headers.remove("x-internal-secret");
            if let Some(provided) = provided {
                headers.insert("x-internal-secret", provided);
            }
            assert_eq!(
                client_address(None, &headers, Some(&expected)).unwrap(),
                Some("192.0.2.10".parse().unwrap())
            );
        }
        headers.insert(
            "x-internal-secret",
            HeaderValue::from_str(expected.expose()).unwrap(),
        );
        assert_eq!(
            client_address(None, &headers, None).unwrap(),
            Some("192.0.2.10".parse().unwrap())
        );
        assert_eq!(
            client_address(None, &headers, Some(&expected)).unwrap(),
            Some("198.51.100.7".parse().unwrap())
        );
        headers.append(
            "x-internal-secret",
            HeaderValue::from_str(expected.expose()).unwrap(),
        );
        assert_eq!(
            client_address(None, &headers, Some(&expected)).unwrap(),
            Some("192.0.2.10".parse().unwrap())
        );
    }

    #[test]
    fn selected_ip_headers_require_one_valid_address() {
        let expected = secret();
        for name in ["cf-connecting-ip", "x-real-client-ip"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                "x-internal-secret",
                HeaderValue::from_str(expected.expose()).unwrap(),
            );
            for value in ["", "invalid", "192.0.2.10, 198.51.100.7"] {
                headers.insert(name, HeaderValue::from_str(value).unwrap());
                assert!(client_address(None, &headers, Some(&expected)).is_err());
            }
            headers.insert(name, HeaderValue::from_static("192.0.2.10"));
            headers.append(name, HeaderValue::from_static("198.51.100.7"));
            assert!(client_address(None, &headers, Some(&expected)).is_err());
        }
    }

    #[test]
    fn untrusted_overrides_and_forwarded_lists_do_not_change_client_identity() {
        let peer = "127.0.0.1".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("x-real-client-ip", HeaderValue::from_static("invalid"));
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("192.0.2.10, 198.51.100.7"),
        );
        assert_eq!(
            client_address(Some(peer), &headers, None).unwrap(),
            Some(peer)
        );
        assert_eq!(client_address(None, &headers, None).unwrap(), None);
        headers.insert(
            "cf-connecting-ip",
            HeaderValue::from_static(" ::ffff:192.0.2.10 "),
        );
        assert_eq!(
            client_address(Some(peer), &headers, None).unwrap(),
            Some("192.0.2.10".parse().unwrap())
        );
    }
}
