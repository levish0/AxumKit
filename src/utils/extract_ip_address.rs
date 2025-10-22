use axum::http::HeaderMap;
use axum_extra::TypedHeader;
use axum_extra::headers::UserAgent;
use std::net::SocketAddr;

pub fn extract_ip_address(headers: &HeaderMap, addr: SocketAddr) -> String {
    headers
        .get("CF-Connecting-IP")
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
