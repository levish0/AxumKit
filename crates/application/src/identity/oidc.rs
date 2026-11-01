pub use super::oidc_native::NativeAuthorization;
use super::{
    AuthenticatedSession, IdentityService, LoginOutcome,
    entities::oauth_flow,
    oidc_provider::{DiscoveredClient, Provider},
    repository::{accounts, oauth, security, sessions},
    service::principal,
    validation::normalize_email,
};
use crate::{
    ApplicationError, ApplicationResult, config::SecretString, database::database_time,
    identity::authentication_operation::AuthenticationOperation,
};
use auth_core::token::{generate_secure_token, hash_token};
use jobs::TraceContext;
use openidconnect::{
    AccessTokenHash, AuthenticationFlow, AuthorizationCode, ClaimsVerificationError, CsrfToken,
    Nonce, NonceVerifier, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, Scope,
    SignatureVerificationError, TokenResponse,
    core::{CoreIdToken, CoreIdTokenClaims, CoreIdTokenVerifier},
    reqwest,
};
use sea_orm::{DatabaseTransaction, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
    time::Duration,
};
use url::Url;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct ProviderSettings {
    pub name: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: SecretString,
    pub redirect_url: Url,
    /// Client IDs of native applications whose provider SDK ID tokens this API accepts.
    /// An empty list disables native ID token login for the provider.
    pub native_client_ids: Vec<String>,
}

impl ProviderSettings {
    pub fn validate(&self) -> ApplicationResult<()> {
        if self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(ApplicationError::InvalidInput("invalid OIDC provider name"));
        }
        if self.client_id.trim().is_empty() || self.client_secret.expose().trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "OIDC provider credentials are required",
            ));
        }
        let mut native_client_ids = BTreeSet::new();
        if self.native_client_ids.iter().any(|client_id| {
            client_id.trim().is_empty() || !native_client_ids.insert(client_id.as_str())
        }) {
            return Err(ApplicationError::InvalidInput(
                "invalid OIDC native client IDs",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct OidcService {
    pub(super) identity: IdentityService,
    pub(super) providers: Arc<BTreeMap<String, Provider>>,
    pub(super) http: reqwest::Client,
}

/// Selects the audience rule for a provider-signed ID token.
#[derive(Clone, Copy)]
pub(super) enum IdTokenAudience {
    /// The token comes from this API's own authorization code exchange.
    BrowserClient,
    /// The token comes from a provider SDK in a native application.
    NativeClients,
}
impl IdTokenAudience {
    fn verifier(self, client: &DiscoveredClient) -> CoreIdTokenVerifier<'_> {
        match self {
            Self::BrowserClient => client.id_token_verifier(),
            // Provider::accepts_native_audiences checks every audience after verification.
            Self::NativeClients => client.id_token_verifier().require_audience_match(false),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FlowSecrets {
    nonce: String,
    pkce_verifier: String,
    session_token_hash: Option<String>,
}

pub struct AuthorizationRedirect {
    pub url: Url,
    pub browser_binding: String,
}
pub enum CallbackOutcome {
    Login(LoginOutcome),
    Linked,
}
pub struct ExternalIdentitySummary {
    pub id: Uuid,
    pub issuer: String,
}

impl OidcService {
    pub async fn discover(
        identity: IdentityService,
        settings: Vec<ProviderSettings>,
    ) -> ApplicationResult<Self> {
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(ApplicationError::internal)?;
        let mut providers = BTreeMap::new();
        for settings in settings {
            settings.validate()?;
            let name = settings.name.clone();
            if providers
                .insert(name, Provider::discover(settings, &http).await?)
                .is_some()
            {
                return Err(ApplicationError::InvalidInput(
                    "duplicate OIDC provider name",
                ));
            }
        }
        Ok(Self {
            identity,
            providers: Arc::new(providers),
            http,
        })
    }

    #[tracing::instrument(skip_all, fields(provider = provider_name))]
    pub async fn begin_authorization(
        &self,
        provider_name: &str,
        actor: Option<&AuthenticatedSession>,
        browser_binding: Option<&str>,
    ) -> ApplicationResult<AuthorizationRedirect> {
        let provider = self
            .providers
            .get(provider_name)
            .ok_or(ApplicationError::NotFound)?;
        let browser_binding = browser_binding
            .filter(|binding| binding.len() == 43)
            .map(str::to_owned)
            .unwrap_or_else(generate_secure_token);
        self.identity
            .limit_authentication_attempts(
                AuthenticationOperation::OidcStart,
                &actor.map_or_else(
                    || hash_token(&browser_binding),
                    |actor| actor.account_id().to_string(),
                ),
            )
            .await?;
        let client = provider.client(&self.http, false).await?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (url, state, nonce) = client
            .authorize_url(
                AuthenticationFlow::<openidconnect::core::CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("email".into()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        let transaction = self.identity.database.begin().await?;
        if let Some(actor) = actor {
            self.identity
                .lock_authenticated_account(&transaction, actor, true)
                .await?;
        }
        let flow_id = Uuid::now_v7();
        let secrets = FlowSecrets {
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
            session_token_hash: actor.map(|actor| actor.token_hash.clone()),
        };
        let encrypted = self.identity.settings.secret_keys.encrypt(
            &flow_context(flow_id),
            &serde_json::to_vec(&secrets).map_err(ApplicationError::internal)?,
        )?;
        oauth::insert_flow(
            &transaction,
            oauth_flow::ActiveModel {
                id: Set(flow_id),
                state_hash: Set(hash_token(state.secret())),
                binding_hash: Set(hash_token(&browser_binding)),
                provider: Set(provider_name.into()),
                secret: Set(encrypted),
                account_id: Set(actor.map(AuthenticatedSession::account_id)),
                session_id: Set(actor.map(AuthenticatedSession::session_id)),
                security_version: Set(actor.map(|actor| actor.security_version)),
                expires_at: Set(database_time(&transaction).await? + chrono::Duration::minutes(5)),
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(AuthorizationRedirect {
            url,
            browser_binding,
        })
    }

    #[tracing::instrument(skip_all, fields(provider = provider_name, request_id = %trace.request_id))]
    pub async fn complete_authorization(
        &self,
        provider_name: &str,
        state: &str,
        browser_binding: &str,
        code: Option<&str>,
        response_issuer: Option<&str>,
        trace: TraceContext,
    ) -> ApplicationResult<CallbackOutcome> {
        if state.len() > 512
            || code.is_some_and(|code| code.len() > 8192)
            || browser_binding.len() != 43
        {
            return Err(ApplicationError::Unauthenticated);
        }
        let provider = self
            .providers
            .get(provider_name)
            .ok_or(ApplicationError::NotFound)?;
        if response_issuer.is_some_and(|issuer| issuer != provider.issuer) {
            return Err(ApplicationError::Unauthenticated);
        }
        let transaction = self.identity.database.begin().await?;
        let flow = oauth::find_flow(&transaction, &hash_token(state))
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
        if flow.provider != provider_name || flow.binding_hash != hash_token(browser_binding) {
            return Err(ApplicationError::Unauthenticated);
        }
        let plaintext = self
            .identity
            .settings
            .secret_keys
            .decrypt(&flow_context(flow.id), &flow.secret)?;
        let secrets: FlowSecrets =
            serde_json::from_slice(&plaintext).map_err(ApplicationError::internal)?;
        // Consume before token exchange: an ambiguous provider response requires a new flow.
        if !oauth::consume_flow(&transaction, flow.id, &flow.binding_hash).await? {
            return Err(ApplicationError::Unauthenticated);
        }
        transaction.commit().await?;
        let code = code.ok_or(ApplicationError::Unauthenticated)?;
        let client = provider.client(&self.http, false).await?;
        let tokens = client
            .exchange_code(AuthorizationCode::new(code.into()))
            .map_err(|_| ApplicationError::Unauthenticated)?
            .set_pkce_verifier(PkceCodeVerifier::new(secrets.pkce_verifier))
            .request_async(&self.http)
            .await
            .map_err(|error| match error {
                openidconnect::RequestTokenError::ServerResponse(response)
                    if *response.error()
                        == openidconnect::core::CoreErrorResponseType::InvalidGrant =>
                {
                    ApplicationError::Unauthenticated
                }
                _ => ApplicationError::ExternalUnavailable,
            })?;
        let id_token = tokens.id_token().ok_or(ApplicationError::Unauthenticated)?;
        let nonce = Nonce::new(secrets.nonce);
        let (claims, client) = self
            .verify_id_token(
                provider,
                client,
                id_token,
                IdTokenAudience::BrowserClient,
                &nonce,
            )
            .await?;
        if let Some(expected_hash) = claims.access_token_hash() {
            let actual_hash = AccessTokenHash::from_token(
                tokens.access_token(),
                id_token
                    .signing_alg()
                    .map_err(|_| ApplicationError::Unauthenticated)?,
                id_token
                    .signing_key(&client.id_token_verifier())
                    .map_err(|_| ApplicationError::Unauthenticated)?,
            )
            .map_err(|_| ApplicationError::Unauthenticated)?;
            if actual_hash != *expected_hash {
                return Err(ApplicationError::Unauthenticated);
            }
        }
        let subject = verified_subject(claims)?;
        let transaction = self.identity.database.begin().await?;
        let outcome = if let Some(account_id) = flow.account_id {
            let account = accounts::lock_account(&transaction, account_id)
                .await?
                .ok_or(ApplicationError::Unauthenticated)?;
            let session = sessions::find_by_id(
                &transaction,
                flow.session_id.ok_or(ApplicationError::Unauthenticated)?,
            )
            .await?
            .ok_or(ApplicationError::Unauthenticated)?;
            let actor = AuthenticatedSession {
                principal: principal(&account),
                session_id: session.id,
                security_version: flow
                    .security_version
                    .ok_or(ApplicationError::Unauthenticated)?,
                token_hash: secrets
                    .session_token_hash
                    .ok_or(ApplicationError::Unauthenticated)?,
            };
            self.identity
                .lock_authenticated_account(&transaction, &actor, true)
                .await?;
            if let Some(existing) =
                oauth::find_identity(&transaction, &provider.issuer, subject).await?
            {
                if existing.account_id != account.id {
                    return Err(ApplicationError::Conflict);
                }
            } else {
                oauth::insert_identity(
                    &transaction,
                    account.id,
                    provider.issuer.clone(),
                    subject.into(),
                )
                .await
                .map_err(identity_conflict)?;
            }
            security::append_audit(
                &transaction,
                Some(account.id),
                "external_identity_linked",
                trace.request_id,
            )
            .await?;
            CallbackOutcome::Linked
        } else {
            CallbackOutcome::Login(
                self.login_with_external_identity(&transaction, provider, subject, claims, trace)
                    .await?,
            )
        };
        transaction.commit().await?;
        Ok(outcome)
    }

    /// Verifies a provider-signed ID token. An unknown signing key refreshes provider keys once.
    pub(super) async fn verify_id_token<'token>(
        &self,
        provider: &Provider,
        client: DiscoveredClient,
        id_token: &'token CoreIdToken,
        audience: IdTokenAudience,
        nonce: impl NonceVerifier + Copy,
    ) -> ApplicationResult<(&'token CoreIdTokenClaims, DiscoveredClient)> {
        let verification = id_token.claims(&audience.verifier(&client), nonce);
        let (claims, client) = match verification {
            Ok(claims) => (claims, client),
            Err(ClaimsVerificationError::SignatureVerification(
                SignatureVerificationError::NoMatchingKey,
            )) => {
                let client = provider.client(&self.http, true).await?;
                let claims = id_token
                    .claims(&audience.verifier(&client), nonce)
                    .map_err(|_| ApplicationError::Unauthenticated)?;
                (claims, client)
            }
            Err(_) => return Err(ApplicationError::Unauthenticated),
        };
        if matches!(audience, IdTokenAudience::NativeClients)
            && !provider.accepts_native_audiences(claims)
        {
            return Err(ApplicationError::Unauthenticated);
        }
        Ok((claims, client))
    }

    /// Returns the primary login outcome for a verified external identity.
    /// A new identity creates an account only for a verified, unused email address.
    pub(super) async fn login_with_external_identity(
        &self,
        transaction: &DatabaseTransaction,
        provider: &Provider,
        subject: &str,
        claims: &CoreIdTokenClaims,
        trace: TraceContext,
    ) -> ApplicationResult<LoginOutcome> {
        let account = if let Some(identity) =
            oauth::find_identity(transaction, &provider.issuer, subject).await?
        {
            let account = accounts::lock_account(transaction, identity.account_id)
                .await?
                .ok_or(ApplicationError::Unauthenticated)?;
            if oauth::find_identity(transaction, &provider.issuer, subject)
                .await?
                .is_none_or(|current| current.id != identity.id || current.account_id != account.id)
            {
                return Err(ApplicationError::Unauthenticated);
            }
            account
        } else {
            if claims.email_verified() != Some(true) {
                return Err(ApplicationError::Unauthenticated);
            }
            let email = normalize_email(
                claims
                    .email()
                    .ok_or(ApplicationError::Unauthenticated)?
                    .as_str(),
            )?;
            if accounts::find_by_email(transaction, &email)
                .await?
                .is_some()
            {
                return Err(ApplicationError::Conflict);
            }
            let account = accounts::insert_account(transaction, email, None, true)
                .await
                .map_err(identity_conflict)?;
            self.identity
                .register_account_notification(
                    transaction,
                    &account,
                    mail::MailPurpose::Welcome,
                    trace,
                )
                .await?;
            oauth::insert_identity(
                transaction,
                account.id,
                provider.issuer.clone(),
                subject.into(),
            )
            .await
            .map_err(identity_conflict)?;
            account
        };
        self.identity
            .complete_primary_authentication(transaction, &account, trace)
            .await
    }

    pub async fn list_external_identities(
        &self,
        actor: &AuthenticatedSession,
    ) -> ApplicationResult<Vec<ExternalIdentitySummary>> {
        let transaction = self.identity.database.begin().await?;
        self.identity
            .lock_authenticated_account(&transaction, actor, false)
            .await?;
        let identities = oauth::list_identities(&transaction, actor.account_id()).await?;
        transaction.commit().await?;
        Ok(identities
            .into_iter()
            .map(|identity| ExternalIdentitySummary {
                id: identity.id,
                issuer: identity.issuer,
            })
            .collect())
    }

    pub async fn unlink_external_identity(
        &self,
        actor: &AuthenticatedSession,
        identity_id: Uuid,
        trace: TraceContext,
    ) -> ApplicationResult<()> {
        let transaction = self.identity.database.begin().await?;
        let account = self
            .identity
            .lock_authenticated_account(&transaction, actor, true)
            .await?;
        let identity = oauth::lock_identity(&transaction, identity_id)
            .await?
            .ok_or(ApplicationError::NotFound)?;
        if identity.account_id != account.id {
            return Err(ApplicationError::NotFound);
        }
        if account.password_hash.is_none()
            && oauth::count_for_account(&transaction, account.id).await? <= 1
        {
            return Err(ApplicationError::Conflict);
        }
        if !oauth::delete_identity(&transaction, identity_id, account.id).await? {
            return Err(ApplicationError::NotFound);
        }
        security::append_audit(
            &transaction,
            Some(account.id),
            "external_identity_unlinked",
            trace.request_id,
        )
        .await?;
        transaction.commit().await?;
        Ok(())
    }
}

fn identity_conflict(error: sea_orm::DbErr) -> ApplicationError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        ApplicationError::Conflict
    } else {
        error.into()
    }
}
pub(super) fn verified_subject(claims: &CoreIdTokenClaims) -> ApplicationResult<&str> {
    let subject = claims.subject().as_str();
    if subject.is_empty() || subject.len() > 255 {
        return Err(ApplicationError::Unauthenticated);
    }
    Ok(subject)
}
fn flow_context(flow_id: Uuid) -> String {
    format!("oidc flow v1:{flow_id}")
}
