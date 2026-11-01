use super::oidc::ProviderSettings;
use crate::{ApplicationError, ApplicationResult};
use openidconnect::{
    ClientId, ClientSecret, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, RedirectUrl,
    core::{CoreClient, CoreIdTokenClaims, CoreProviderMetadata},
    reqwest,
};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub(super) type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;
pub(super) struct Provider {
    pub issuer: String,
    settings: ProviderSettings,
    cache: Mutex<ClientCache>,
}
struct ClientCache {
    client: DiscoveredClient,
    loaded_at: Instant,
    refresh_retry_after: Option<Instant>,
    refresh_failed: bool,
}

struct ClientRefresh<'a>(&'a mut ClientCache);

impl Drop for ClientRefresh<'_> {
    fn drop(&mut self) {
        // Start the cooldown when refresh ends, including failure or cancellation.
        self.0.refresh_retry_after = Some(Instant::now() + Duration::from_secs(5));
    }
}

impl Provider {
    pub async fn discover(
        settings: ProviderSettings,
        http: &reqwest::Client,
    ) -> ApplicationResult<Self> {
        let client = discover_client(&settings, http).await?;
        Ok(Self {
            issuer: settings.issuer.clone(),
            settings,
            cache: Mutex::new(ClientCache {
                client,
                loaded_at: Instant::now(),
                refresh_retry_after: None,
                refresh_failed: false,
            }),
        })
    }

    pub fn supports_native_login(&self) -> bool {
        !self.settings.native_client_ids.is_empty()
    }

    /// Native SDK tokens can name the web client or a configured native client. Every
    /// audience and any authorized party must be one of them. Multiple audiences require
    /// an authorized party.
    pub fn accepts_native_audiences(&self, claims: &CoreIdTokenClaims) -> bool {
        let configured = |client_id: &str| {
            client_id == self.settings.client_id
                || self
                    .settings
                    .native_client_ids
                    .iter()
                    .any(|native_client_id| native_client_id == client_id)
        };
        let audiences = claims.audiences();
        !audiences.is_empty()
            && audiences.iter().all(|audience| configured(audience))
            && claims
                .authorized_party()
                .map_or(audiences.len() == 1, |party| configured(party))
    }

    pub async fn client(
        &self,
        http: &reqwest::Client,
        unknown_signing_key: bool,
    ) -> ApplicationResult<DiscoveredClient> {
        let mut cache = self.cache.lock().await;
        let refresh = cache.loaded_at.elapsed() >= Duration::from_secs(300)
            || unknown_signing_key
            || cache.refresh_failed;
        if refresh {
            if cache
                .refresh_retry_after
                .is_some_and(|retry_after| Instant::now() < retry_after)
            {
                return if cache.refresh_failed {
                    Err(ApplicationError::ExternalUnavailable)
                } else {
                    Ok(cache.client.clone())
                };
            }
            cache.refresh_failed = true;
            let refresh = ClientRefresh(&mut cache);
            refresh.0.client = discover_client(&self.settings, http).await?;
            refresh.0.loaded_at = Instant::now();
            refresh.0.refresh_failed = false;
        }
        Ok(cache.client.clone())
    }
}

async fn discover_client(
    settings: &ProviderSettings,
    http: &reqwest::Client,
) -> ApplicationResult<DiscoveredClient> {
    let metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(settings.issuer.clone()).map_err(ApplicationError::internal)?,
        http,
    )
    .await
    .map_err(|_| ApplicationError::ExternalUnavailable)?;
    Ok(CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(settings.client_id.clone()),
        Some(ClientSecret::new(settings.client_secret.expose().into())),
    )
    .set_redirect_uri(
        RedirectUrl::new(settings.redirect_url.to_string()).map_err(ApplicationError::internal)?,
    ))
}
