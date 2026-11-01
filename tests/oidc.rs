mod support;

#[path = "oidc/initial_password.rs"]
mod initial_password;
#[path = "oidc/native.rs"]
mod native;
#[path = "oidc/providers.rs"]
mod providers;

use anyhow::{Context, Result, ensure};
use application::{
    ApplicationError,
    config::SecretString,
    identity::{
        LoginOutcome, MfaProof,
        oidc::{AuthorizationRedirect, CallbackOutcome, OidcService, ProviderSettings},
    },
};
use axum::{
    Form, Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use jobs::TraceContext;
use openidconnect::{
    Audience, EmptyAdditionalClaims, EndUserEmail, IssuerUrl, JsonWebKeyId, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, PrivateSigningKey, StandardClaims, SubjectIdentifier,
    core::{
        CoreIdToken, CoreIdTokenClaims, CoreJsonWebKeySet, CoreJwsSigningAlgorithm,
        CoreRsaPrivateSigningKey,
    },
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};
use test_support::TestDatabase;
use tokio::task::JoinHandle;

struct MockProvider {
    state: Arc<ProviderState>,
    task: JoinHandle<()>,
}
struct ProviderState {
    issuer: String,
    keys: [CoreRsaPrivateSigningKey; 2],
    rotated: AtomicBool,
    discovery_requests: AtomicUsize,
    discovery_delay_milliseconds: AtomicU64,
    discovery_unavailable: AtomicBool,
    codes: Mutex<BTreeMap<String, CodeClaims>>,
}
struct CodeClaims {
    nonce: String,
    pkce: String,
    email: String,
    subject: String,
    wrong_nonce: bool,
    unavailable: bool,
}
impl MockProvider {
    async fn start() -> Result<Self> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let issuer = format!("http://{}", listener.local_addr()?);
        let keys = [
            CoreRsaPrivateSigningKey::from_pem(
                include_str!("fixtures/oidc-test-key.pem"),
                Some(JsonWebKeyId::new("first".into())),
            )
            .map_err(anyhow::Error::msg)?,
            CoreRsaPrivateSigningKey::from_pem(
                include_str!("fixtures/oidc-rotated-test-key.pem"),
                Some(JsonWebKeyId::new("rotated".into())),
            )
            .map_err(anyhow::Error::msg)?,
        ];
        let state = Arc::new(ProviderState {
            issuer,
            keys,
            rotated: AtomicBool::new(false),
            discovery_requests: AtomicUsize::new(0),
            discovery_delay_milliseconds: AtomicU64::new(0),
            discovery_unavailable: AtomicBool::new(false),
            codes: Mutex::new(BTreeMap::new()),
        });
        let router = Router::new()
            .route("/.well-known/openid-configuration", get(discovery))
            .route("/jwks", get(jwks))
            .route("/token", post(exchange))
            .with_state(state.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        Ok(Self { state, task })
    }
    fn settings(&self) -> ProviderSettings {
        ProviderSettings {
            name: "test".into(),
            issuer: self.state.issuer.clone(),
            client_id: "test-client".into(),
            client_secret: SecretString::new("public-test-client-secret"),
            redirect_url: "http://localhost:3000/api/v1/auth/oidc/test/callback"
                .parse()
                .unwrap(),
            native_client_ids: Vec::new(),
        }
    }
    fn authorize(
        &self,
        redirect: &AuthorizationRedirect,
        email: &str,
        subject: &str,
        wrong_nonce: bool,
        unavailable: bool,
    ) -> Result<(String, String)> {
        let query: BTreeMap<_, _> = redirect
            .url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .collect();
        ensure!(
            query.get("code_challenge_method").map(String::as_str) == Some("S256"),
            "PKCE S256 missing"
        );
        let code = auth_core::token::generate_secure_token();
        self.state.codes.lock().unwrap().insert(
            code.clone(),
            CodeClaims {
                nonce: query.get("nonce").context("nonce missing")?.clone(),
                pkce: query.get("code_challenge").context("PKCE missing")?.clone(),
                email: email.into(),
                subject: subject.into(),
                wrong_nonce,
                unavailable,
            },
        );
        Ok((query.get("state").context("state missing")?.clone(), code))
    }
}
impl Drop for MockProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn discovery(State(state): State<Arc<ProviderState>>) -> Response {
    state.discovery_requests.fetch_add(1, Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(
        state.discovery_delay_milliseconds.load(Ordering::SeqCst),
    ))
    .await;
    if state.discovery_unavailable.load(Ordering::SeqCst) {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    Json(
        json!({"issuer":state.issuer,"authorization_endpoint":format!("{}/authorize",state.issuer),"token_endpoint":format!("{}/token",state.issuer),"jwks_uri":format!("{}/jwks",state.issuer),"response_types_supported":["code"],"subject_types_supported":["public"],"id_token_signing_alg_values_supported":["RS256"],"token_endpoint_auth_methods_supported":["client_secret_basic"],"code_challenge_methods_supported":["S256"]}),
    )
    .into_response()
}
async fn jwks(State(state): State<Arc<ProviderState>>) -> Json<CoreJsonWebKeySet> {
    let index = usize::from(state.rotated.load(Ordering::SeqCst));
    Json(CoreJsonWebKeySet::new(vec![
        state.keys[index].as_verification_key(),
    ]))
}
async fn exchange(
    State(state): State<Arc<ProviderState>>,
    Form(form): Form<BTreeMap<String, String>>,
) -> Response {
    let claims = form
        .get("code")
        .and_then(|code| state.codes.lock().unwrap().remove(code));
    let Some(claims) = claims else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        )
            .into_response();
    };
    if claims.unavailable {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let pkce = form.get("code_verifier").map(|verifier| {
        PkceCodeChallenge::from_code_verifier_sha256(&PkceCodeVerifier::new(verifier.clone()))
    });
    if pkce.is_none_or(|pkce| pkce.as_str() != claims.pkce) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"invalid_grant"})),
        )
            .into_response();
    }
    let nonce = if claims.wrong_nonce {
        "wrong-nonce".into()
    } else {
        claims.nonce
    };
    let claims = CoreIdTokenClaims::new(
        IssuerUrl::new(state.issuer.clone()).unwrap(),
        vec![Audience::new("test-client".into())],
        chrono::Utc::now() + chrono::Duration::minutes(5),
        chrono::Utc::now(),
        StandardClaims::new(SubjectIdentifier::new(claims.subject))
            .set_email(Some(EndUserEmail::new(claims.email)))
            .set_email_verified(Some(true)),
        EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(nonce)));
    let index = usize::from(state.rotated.load(Ordering::SeqCst));
    let token = CoreIdToken::new(
        claims,
        &state.keys[index],
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap();
    Json(json!({"access_token":"public-mock-access-token","token_type":"Bearer","id_token":token.to_string()})).into_response()
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn slow_discovery_failure_and_cancellation_both_limit_subsequent_refreshes() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let oidc = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let redirect = oidc.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(
        &redirect,
        "refresh@example.test",
        "refresh-subject",
        false,
        false,
    )?;
    provider.state.rotated.store(true, Ordering::SeqCst);
    provider
        .state
        .discovery_delay_milliseconds
        .store(6000, Ordering::SeqCst);
    provider
        .state
        .discovery_unavailable
        .store(true, Ordering::SeqCst);
    assert!(matches!(
        oidc.complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            None,
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::ExternalUnavailable)
    ));
    let after_failure = provider.state.discovery_requests.load(Ordering::SeqCst);
    provider
        .state
        .discovery_delay_milliseconds
        .store(0, Ordering::SeqCst);
    provider
        .state
        .discovery_unavailable
        .store(false, Ordering::SeqCst);
    for _ in 0..3 {
        assert!(matches!(
            oidc.begin_authorization("test", None, None).await,
            Err(ApplicationError::ExternalUnavailable)
        ));
    }
    assert_eq!(
        provider.state.discovery_requests.load(Ordering::SeqCst),
        after_failure
    );
    tokio::time::sleep(Duration::from_millis(5100)).await;
    oidc.begin_authorization("test", None, None).await?;
    assert_eq!(
        provider.state.discovery_requests.load(Ordering::SeqCst),
        after_failure + 1
    );

    let cancellable = OidcService::discover(identity, vec![provider.settings()]).await?;
    let redirect = cancellable.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(
        &redirect,
        "refresh@example.test",
        "refresh-subject",
        false,
        false,
    )?;
    provider.state.rotated.store(false, Ordering::SeqCst);
    provider
        .state
        .discovery_delay_milliseconds
        .store(6000, Ordering::SeqCst);
    let before_cancellation = provider.state.discovery_requests.load(Ordering::SeqCst);
    let callback_service = cancellable.clone();
    let callback = tokio::spawn(async move {
        callback_service
            .complete_authorization(
                "test",
                &state,
                &redirect.browser_binding,
                Some(&code),
                None,
                TraceContext::default(),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        while provider.state.discovery_requests.load(Ordering::SeqCst) == before_cancellation {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    callback.abort();
    assert!(matches!(callback.await, Err(error) if error.is_cancelled()));
    assert!(matches!(
        cancellable.begin_authorization("test", None, None).await,
        Err(ApplicationError::ExternalUnavailable)
    ));
    assert_eq!(
        provider.state.discovery_requests.load(Ordering::SeqCst),
        before_cancellation + 1
    );
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn oidc_binding_single_use_nonce_key_rotation_and_provider_failure() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let first = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let second = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let redirect = first.begin_authorization("test", None, None).await?;
    let (state, code) =
        provider.authorize(&redirect, "oidc@example.test", "subject-one", false, false)?;
    let wrong_binding = auth_core::token::generate_secure_token();
    assert!(matches!(
        second
            .complete_authorization(
                "test",
                &state,
                &wrong_binding,
                Some(&code),
                None,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    provider.state.rotated.store(true, Ordering::SeqCst);
    let before = provider.state.discovery_requests.load(Ordering::SeqCst);
    let outcome = second
        .complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            Some(&provider.state.issuer),
            TraceContext::default(),
        )
        .await?;
    assert!(matches!(
        outcome,
        CallbackOutcome::Login(LoginOutcome::Session(_))
    ));
    assert!(provider.state.discovery_requests.load(Ordering::SeqCst) > before);
    assert!(matches!(
        first
            .complete_authorization(
                "test",
                &state,
                &redirect.browser_binding,
                Some(&code),
                None,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    for (wrong_nonce, unavailable) in [(true, false), (false, true)] {
        let redirect = second.begin_authorization("test", None, None).await?;
        let (state, code) = provider.authorize(
            &redirect,
            "oidc@example.test",
            "subject-one",
            wrong_nonce,
            unavailable,
        )?;
        let result = second
            .complete_authorization(
                "test",
                &state,
                &redirect.browser_binding,
                Some(&code),
                None,
                TraceContext::default(),
            )
            .await;
        assert!(if unavailable {
            matches!(result, Err(ApplicationError::ExternalUnavailable))
        } else {
            matches!(result, Err(ApplicationError::Unauthenticated))
        });
        assert!(matches!(
            second
                .complete_authorization(
                    "test",
                    &state,
                    &redirect.browser_binding,
                    Some(&code),
                    None,
                    TraceContext::default()
                )
                .await,
            Err(ApplicationError::Unauthenticated)
        ));
    }
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn explicit_linking_does_not_merge_email_and_oidc_cannot_bypass_mfa() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (actor, _) = database.verified_account(&identity).await?;
    let email = identity.get_account(&actor).await?.email;
    let provider = MockProvider::start().await?;
    let oidc = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let redirect = oidc.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(&redirect, &email, "linked-subject", false, false)?;
    assert!(matches!(
        oidc.complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            None,
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::Conflict)
    ));
    let redirect = oidc.begin_authorization("test", Some(&actor), None).await?;
    let (state, code) = provider.authorize(&redirect, &email, "linked-subject", false, false)?;
    assert!(matches!(
        oidc.complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            None,
            TraceContext::default()
        )
        .await?,
        CallbackOutcome::Linked
    ));
    let enrollment = identity.begin_totp_enrollment(&actor).await?;
    let totp = totp_rs::Totp::from_url(&enrollment.authenticator_url)?;
    let recovery_codes = identity
        .confirm_totp_enrollment(
            &actor,
            &enrollment.challenge,
            &totp.generate_current().to_string(),
            TraceContext::default(),
        )
        .await?;
    identity
        .set_administrator_by_operator(actor.account_id(), true, TraceContext::default())
        .await?;
    let redirect = oidc.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(&redirect, &email, "linked-subject", false, false)?;
    let CallbackOutcome::Login(LoginOutcome::MfaRequired { challenge }) = oidc
        .complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            None,
            TraceContext::default(),
        )
        .await?
    else {
        anyhow::bail!("OIDC bypassed MFA");
    };
    let credentials = identity
        .complete_mfa_login(
            &challenge,
            MfaProof::RecoveryCode(recovery_codes[0].clone()),
            TraceContext::default(),
        )
        .await?;
    assert_eq!(
        identity
            .authenticate_session(&credentials.token)
            .await?
            .account_id(),
        actor.account_id()
    );
    use application::identity::entities::account;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};
    account::Entity::update_many()
        .col_expr(
            account::Column::TotpSecret,
            Expr::value(Option::<String>::None),
        )
        .filter(account::Column::Id.eq(actor.account_id()))
        .exec(&database.connection)
        .await?;
    let redirect = oidc.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(&redirect, &email, "linked-subject", false, false)?;
    assert!(matches!(
        oidc.complete_authorization(
            "test",
            &state,
            &redirect.browser_binding,
            Some(&code),
            None,
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::Unauthenticated)
    ));
    database.finish().await
}
