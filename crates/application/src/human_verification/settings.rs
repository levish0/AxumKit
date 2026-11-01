use crate::{ApplicationError, ApplicationResult, config::SecretString};
use std::time::Duration;
use url::Url;

pub const TURNSTILE_SITEVERIFY_URL: &str =
    "https://challenges.cloudflare.com/turnstile/v0/siteverify";

/// Cloudflare publishes these secret keys for tests. They always pass, always
/// fail, or report a spent token, so production configuration rejects them.
pub const TURNSTILE_TEST_SECRET_KEYS: [&str; 3] = [
    "1x0000000000000000000000000000000AA",
    "2x0000000000000000000000000000000AA",
    "3x0000000000000000000000000000000AA",
];

#[derive(Clone, Debug)]
pub struct TurnstileSettings {
    pub secret_key: SecretString,
    /// Environment configuration always uses [`TURNSTILE_SITEVERIFY_URL`].
    /// Tests replace it with a local server.
    pub siteverify_url: Url,
    /// Accepted challenge hostnames. An empty list skips the hostname check.
    pub expected_hostnames: Vec<String>,
    /// Requires the widget action to equal the protected operation's action.
    pub verify_action: bool,
    /// Deadline for each siteverify attempt, including the response body.
    pub attempt_timeout: Duration,
}

impl TurnstileSettings {
    pub fn validate(&self) -> ApplicationResult<()> {
        let valid_url = matches!(self.siteverify_url.scheme(), "https" | "http")
            && self.siteverify_url.host_str().is_some()
            && self.siteverify_url.username().is_empty()
            && self.siteverify_url.password().is_none()
            && self.siteverify_url.query().is_none()
            && self.siteverify_url.fragment().is_none();
        let valid_hostnames = self.expected_hostnames.iter().all(|hostname| {
            !hostname.is_empty()
                && hostname.len() <= 253
                && hostname
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        });
        if self.secret_key.expose().trim().is_empty()
            || !valid_url
            || !valid_hostnames
            || self.attempt_timeout.is_zero()
            || self.attempt_timeout > Duration::from_secs(30)
        {
            return Err(ApplicationError::InvalidInput("invalid Turnstile settings"));
        }
        Ok(())
    }
}
