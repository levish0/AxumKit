use axum::http::{HeaderMap, HeaderValue};

pub(super) fn quota_headers(quota: &application::api_access::QuotaStatus) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for (name, value) in [
        ("x-ratelimit-limit", quota.minute_limit),
        (
            "x-ratelimit-remaining",
            quota.minute_limit - quota.minute_used,
        ),
        ("x-ratelimit-reset", quota.minute_resets_at.timestamp()),
        ("x-quota-limit", quota.month_limit),
        ("x-quota-remaining", quota.month_limit - quota.month_used),
        ("x-quota-reset", quota.month_resets_at.timestamp()),
    ] {
        headers.insert(
            name,
            HeaderValue::from_str(&value.to_string()).expect("integer header"),
        );
    }
    headers
}
