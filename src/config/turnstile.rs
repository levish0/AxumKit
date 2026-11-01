use super::number;
use anyhow::{Result, ensure};
use application::{
    config::SecretString,
    human_verification::{TURNSTILE_SITEVERIFY_URL, TURNSTILE_TEST_SECRET_KEYS, TurnstileSettings},
};
use std::{collections::BTreeMap, time::Duration};
use url::Url;

const TURNSTILE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn parse_turnstile(
    values: &BTreeMap<String, String>,
    production: bool,
) -> Result<Option<TurnstileSettings>> {
    let supplied = |name: &str| {
        values
            .get(name)
            .is_some_and(|value| !value.trim().is_empty())
    };
    let Some(secret_key) = values
        .get("TURNSTILE_SECRET_KEY")
        .filter(|value| !value.trim().is_empty())
    else {
        ensure!(
            !["TURNSTILE_EXPECTED_HOSTNAMES", "TURNSTILE_VERIFY_ACTION"]
                .iter()
                .any(|name| supplied(name)),
            "Turnstile options require TURNSTILE_SECRET_KEY"
        );
        return Ok(None);
    };
    ensure!(
        !production || !TURNSTILE_TEST_SECRET_KEYS.contains(&secret_key.as_str()),
        "production cannot use a Cloudflare Turnstile test secret key"
    );
    let expected_hostnames = if supplied("TURNSTILE_EXPECTED_HOSTNAMES") {
        values["TURNSTILE_EXPECTED_HOSTNAMES"]
            .split(',')
            .map(|hostname| hostname.trim().to_ascii_lowercase())
            .collect()
    } else {
        Vec::new()
    };
    let settings = TurnstileSettings {
        secret_key: SecretString::new(secret_key.clone()),
        siteverify_url: Url::parse(TURNSTILE_SITEVERIFY_URL)?,
        expected_hostnames,
        verify_action: number(values, "TURNSTILE_VERIFY_ACTION", true)?,
        attempt_timeout: TURNSTILE_ATTEMPT_TIMEOUT,
    };
    settings.validate()?;
    Ok(Some(settings))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generated at runtime so tests keep credentials out of source literals.
    fn private_secret() -> String {
        format!("private-{}", uuid::Uuid::now_v7().simple())
    }

    fn values(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into()))
            .collect()
    }

    #[test]
    fn turnstile_is_optional_and_parses_explicit_checks() {
        let secret = private_secret();
        assert!(parse_turnstile(&BTreeMap::new(), true).unwrap().is_none());
        assert!(
            parse_turnstile(&values(&[("TURNSTILE_SECRET_KEY", " ")]), true)
                .unwrap()
                .is_none()
        );
        let settings = parse_turnstile(
            &values(&[
                ("TURNSTILE_SECRET_KEY", secret.as_str()),
                (
                    "TURNSTILE_EXPECTED_HOSTNAMES",
                    "App.Example.com, www.example.com",
                ),
            ]),
            true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(settings.siteverify_url.as_str(), TURNSTILE_SITEVERIFY_URL);
        assert_eq!(
            settings.expected_hostnames,
            ["app.example.com", "www.example.com"]
        );
        assert!(settings.verify_action);
        assert_eq!(settings.attempt_timeout, TURNSTILE_ATTEMPT_TIMEOUT);
        assert!(!format!("{settings:?}").contains(&secret));
        let settings = parse_turnstile(
            &values(&[
                ("TURNSTILE_SECRET_KEY", TURNSTILE_TEST_SECRET_KEYS[0]),
                ("TURNSTILE_VERIFY_ACTION", "false"),
            ]),
            false,
        )
        .unwrap()
        .unwrap();
        assert!(settings.expected_hostnames.is_empty());
        assert!(!settings.verify_action);
    }

    #[test]
    fn invalid_turnstile_configuration_fails_without_the_secret() {
        let secret = private_secret();
        for entries in [
            vec![("TURNSTILE_EXPECTED_HOSTNAMES", "app.example.com")],
            vec![("TURNSTILE_VERIFY_ACTION", "true")],
            vec![
                ("TURNSTILE_SECRET_KEY", secret.as_str()),
                (
                    "TURNSTILE_EXPECTED_HOSTNAMES",
                    "app.example.com,,www.example.com",
                ),
            ],
            vec![
                ("TURNSTILE_SECRET_KEY", secret.as_str()),
                ("TURNSTILE_EXPECTED_HOSTNAMES", "https://app.example.com"),
            ],
            vec![
                ("TURNSTILE_SECRET_KEY", secret.as_str()),
                ("TURNSTILE_VERIFY_ACTION", "sometimes"),
            ],
        ] {
            let error = parse_turnstile(&values(&entries), true).unwrap_err();
            assert!(!format!("{error:#}").contains(&secret));
        }
        for test_secret in TURNSTILE_TEST_SECRET_KEYS {
            assert!(
                parse_turnstile(&values(&[("TURNSTILE_SECRET_KEY", test_secret)]), true).is_err()
            );
        }
    }
}
