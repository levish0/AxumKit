use super::validate_web_url;
use anyhow::{Context, Result, ensure};
use application::{config::SecretString, identity::oidc::ProviderSettings};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use url::Url;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredProvider {
    name: String,
    issuer: String,
    client_id: String,
    client_secret: String,
    redirect_url: String,
    #[serde(default)]
    native_client_ids: Vec<String>,
}

pub(super) fn parse_providers(
    values: &BTreeMap<String, String>,
    production: bool,
) -> Result<Vec<ProviderSettings>> {
    ensure!(
        ![
            "OIDC_PROVIDER",
            "OIDC_ISSUER",
            "OIDC_CLIENT_ID",
            "OIDC_CLIENT_SECRET",
            "OIDC_REDIRECT_URL",
        ]
        .iter()
        .any(|name| values.get(*name).is_some_and(|value| !value.is_empty())),
        "use OIDC_PROVIDERS instead of single-provider OIDC settings"
    );
    let Some(encoded) = values
        .get("OIDC_PROVIDERS")
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Vec::new());
    };
    // Deserialization errors can include submitted client secrets.
    let configured: Vec<ConfiguredProvider> = serde_json::from_str(encoded)
        .map_err(|_| anyhow::anyhow!("invalid OIDC_PROVIDERS JSON array"))?;
    let mut names = BTreeSet::new();
    configured
        .into_iter()
        .map(|provider| {
            let issuer = Url::parse(&provider.issuer).context("invalid OIDC provider issuer")?;
            let redirect_url =
                Url::parse(&provider.redirect_url).context("invalid OIDC provider redirect URL")?;
            validate_web_url(&issuer, production)?;
            validate_web_url(&redirect_url, production)?;
            let settings = ProviderSettings {
                name: provider.name,
                issuer: provider.issuer,
                client_id: provider.client_id,
                client_secret: SecretString::new(provider.client_secret),
                redirect_url,
                native_client_ids: provider.native_client_ids,
            };
            settings.validate()?;
            ensure!(
                names.insert(settings.name.clone()),
                "duplicate OIDC provider name"
            );
            Ok(settings)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn provider(name: &str) -> Value {
        json!({
            "name": name,
            "issuer": format!("https://{name}.example.test"),
            "client_id": format!("{name}-client"),
            "client_secret": "private-provider-test-secret",
            "redirect_url": format!("https://app.example.test/api/v1/auth/oidc/{name}/callback")
        })
    }

    fn values(providers: Value) -> BTreeMap<String, String> {
        BTreeMap::from([("OIDC_PROVIDERS".into(), providers.to_string())])
    }

    #[test]
    fn multiple_providers_are_explicit_and_secrets_are_redacted() {
        let settings = parse_providers(
            &values(json!([provider("first"), provider("second")])),
            true,
        )
        .unwrap();
        assert_eq!(settings.len(), 2);
        assert_eq!(settings[0].name, "first");
        assert_eq!(settings[1].client_id, "second-client");
        assert_eq!(
            settings[1].redirect_url.path(),
            "/api/v1/auth/oidc/second/callback"
        );
        assert!(!format!("{settings:?}").contains("private-provider-test-secret"));
        assert!(settings[0].native_client_ids.is_empty());
        let mut native = provider("native");
        native["native_client_ids"] = json!(["com.example.ios", "android-client"]);
        let settings = parse_providers(&values(json!([native])), true).unwrap();
        assert_eq!(
            settings[0].native_client_ids,
            ["com.example.ios", "android-client"]
        );
        assert!(parse_providers(&BTreeMap::new(), true).unwrap().is_empty());
        assert!(
            parse_providers(&values(json!([])), true)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn malformed_or_ambiguous_configuration_fails_without_secret_values() {
        let mut missing_secret = provider("first");
        missing_secret
            .as_object_mut()
            .unwrap()
            .remove("client_secret");
        let mut empty_secret = provider("first");
        empty_secret["client_secret"] = json!("");
        let mut invalid_name = provider("first");
        invalid_name["name"] = json!("invalid/name");
        let mut empty_client_id = provider("first");
        empty_client_id["client_id"] = json!(" ");
        let mut unknown_field = provider("first");
        unknown_field["private-provider-test-secret"] = json!(true);
        let mut invalid_issuer = provider("first");
        invalid_issuer["issuer"] = json!("http://provider.example.test");
        let mut invalid_redirect = provider("first");
        invalid_redirect["redirect_url"] = json!("https://app.example.test/callback?secret=value");
        let mut empty_native_client_id = provider("first");
        empty_native_client_id["native_client_ids"] = json!([" "]);
        let mut duplicate_native_client_id = provider("first");
        duplicate_native_client_id["native_client_ids"] = json!(["native", "native"]);
        for providers in [
            json!([provider("first"), provider("first")]),
            json!([missing_secret]),
            json!([empty_secret]),
            json!([invalid_name]),
            json!([empty_client_id]),
            json!([unknown_field]),
            json!([invalid_issuer]),
            json!([invalid_redirect]),
            json!([empty_native_client_id]),
            json!([duplicate_native_client_id]),
            json!({"private-provider-test-secret": true}),
        ] {
            let error = parse_providers(&values(providers), true).unwrap_err();
            assert!(!format!("{error:#}").contains("private-provider-test-secret"));
        }
        let legacy = BTreeMap::from([("OIDC_ISSUER".into(), "https://issuer.example.test".into())]);
        assert!(parse_providers(&legacy, true).is_err());
    }
}
