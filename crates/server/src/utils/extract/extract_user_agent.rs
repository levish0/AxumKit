use axum::http::HeaderMap;
use axum::http::header::USER_AGENT;
use axum_extra::TypedHeader;
use axum_extra::headers::UserAgent;

pub fn extract_user_agent(user_agent: Option<TypedHeader<UserAgent>>) -> Option<String> {
    user_agent
        .map(|ua| ua.0.to_string())
        .filter(|ua| !ua.trim().is_empty())
}

/// Extract the `User-Agent` from a raw `HeaderMap` (for handlers that already read `headers`,
/// e.g. to extract the client IP, and don't take the typed-header extractor).
pub fn extract_user_agent_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|ua| ua.trim().to_string())
        .filter(|ua| !ua.is_empty())
}
