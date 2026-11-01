use anyhow::{Result, ensure};
use application::config::SecretString;
use std::collections::BTreeMap;

pub(super) fn parse_internal_proxy_secret(
    values: &BTreeMap<String, String>,
) -> Result<Option<SecretString>> {
    ensure!(
        !values.contains_key("TRUSTED_PROXY_IPS"),
        "TRUSTED_PROXY_IPS is no longer supported; use the Cloudflare Tunnel proxy contract"
    );
    let Some(secret) = values
        .get("INTERNAL_PROXY_SECRET")
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    ensure!(
        secret.len() >= 32 && secret.bytes().all(|byte| byte.is_ascii_graphic()),
        "INTERNAL_PROXY_SECRET must contain at least 32 visible ASCII characters"
    );
    Ok(Some(SecretString::new(secret.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_secret_is_optional_and_redacted() {
        let mut values = BTreeMap::new();
        assert!(parse_internal_proxy_secret(&values).unwrap().is_none());
        values.insert("INTERNAL_PROXY_SECRET".into(), String::new());
        assert!(parse_internal_proxy_secret(&values).unwrap().is_none());
        let secret = auth_core::token::generate_secure_token();
        values.insert("INTERNAL_PROXY_SECRET".into(), secret.clone());
        let parsed = parse_internal_proxy_secret(&values).unwrap().unwrap();
        assert_eq!(parsed.expose(), secret);
        assert!(!format!("{parsed:?}").contains(&secret));
    }

    #[test]
    fn invalid_proxy_configuration_never_includes_the_secret_in_errors() {
        for secret in [
            "short".to_owned(),
            " ".repeat(32),
            "\n".repeat(32),
            "é".repeat(32),
        ] {
            let values = BTreeMap::from([("INTERNAL_PROXY_SECRET".into(), secret.clone())]);
            let error = parse_internal_proxy_secret(&values).unwrap_err();
            assert!(!format!("{error:#}").contains(&secret));
        }
        let legacy = BTreeMap::from([("TRUSTED_PROXY_IPS".into(), "127.0.0.1".into())]);
        assert!(parse_internal_proxy_secret(&legacy).is_err());
    }
}
