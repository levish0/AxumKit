use axum::http::HeaderMap;
use std::net::SocketAddr;

/// Extract real client IP address with proxy support
///
/// Priority order:
/// 1. CF-Connecting-IP (Cloudflare's trusted header)
/// 2. X-Real-IP (Nginx, other reverse proxies)
/// 3. X-Forwarded-For (Standard proxy header, uses first/leftmost IP)
/// 4. ConnectInfo (Direct connection socket address)
pub fn extract_ip_address(headers: &HeaderMap, addr: SocketAddr) -> String {
    headers
        .get("CF-Connecting-IP")
        .or_else(|| headers.get("X-Real-IP"))
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
        .or_else(|| {
            headers
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| addr.ip().to_string())
}
