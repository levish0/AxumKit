//! Device-recognition cookie for new-device login verification (OWASP ASVS 6.3.5).
//!
//! The cookie carries a raw opaque device token; the server stores only its hash in
//! `known_devices`. A returning browser presents the cookie so it is not re-challenged. This is a
//! browser handle, not a credential — a forged/unknown value simply fails to match any
//! `known_devices` row and triggers a fresh email challenge (fail-safe).

use config::ServerConfig;
use cookie::{Cookie, SameSite, time::Duration};

/// Days a device cookie persists. Intentionally long: the point is to recognize a returning
/// browser. Server-side trust lives in the `known_devices` row; the cookie is just the handle.
const DEVICE_COOKIE_MAX_AGE_DAYS: i64 = 400;

/// Device cookie name, with the same name-prefix hardening as the session cookie
/// (`__Host-`/`__Secure-` in production, plain on dev HTTP).
pub fn device_cookie_name() -> String {
    let config = ServerConfig::get();
    if config.is_dev {
        "device_id".to_string()
    } else if config.cookie_domain.is_some() {
        "__Secure-device_id".to_string()
    } else {
        "__Host-device_id".to_string()
    }
}

/// Build the long-lived device-recognition cookie carrying `token`.
pub fn build_device_cookie(token: String) -> Cookie<'static> {
    let config = ServerConfig::get();
    let is_dev = config.is_dev;
    let same_site = if is_dev {
        SameSite::None
    } else {
        SameSite::Lax
    };

    let mut builder = Cookie::build((device_cookie_name(), token))
        .http_only(true)
        .secure(true)
        .same_site(same_site)
        .path("/")
        .max_age(Duration::days(DEVICE_COOKIE_MAX_AGE_DAYS));

    // `__Host-` requires no Domain; only set a domain (→ `__Secure-`) when configured.
    if !is_dev && let Some(ref domain) = config.cookie_domain {
        builder = builder.domain(domain);
    }

    builder.build()
}
