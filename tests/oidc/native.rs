use super::*;
use openidconnect::ClientId;

/// Builds a signed token for a flow nonce.
type TokenForNonce = fn(&MockProvider, &str) -> String;

fn native_settings(provider: &MockProvider, name: &str) -> ProviderSettings {
    let mut settings = provider.settings();
    settings.name = name.into();
    settings.native_client_ids = vec!["native-ios".into(), "native-android".into()];
    settings
}

fn native_claims(
    provider: &MockProvider,
    audiences: &[&str],
    nonce: &str,
    email: &str,
    subject: &str,
) -> CoreIdTokenClaims {
    claims_with_email_verification(provider, audiences, nonce, email, subject, true)
}

fn claims_with_email_verification(
    provider: &MockProvider,
    audiences: &[&str],
    nonce: &str,
    email: &str,
    subject: &str,
    email_verified: bool,
) -> CoreIdTokenClaims {
    CoreIdTokenClaims::new(
        IssuerUrl::new(provider.state.issuer.clone()).unwrap(),
        audiences
            .iter()
            .map(|audience| Audience::new((*audience).into()))
            .collect(),
        chrono::Utc::now() + chrono::Duration::minutes(5),
        chrono::Utc::now(),
        StandardClaims::new(SubjectIdentifier::new(subject.into()))
            .set_email(Some(EndUserEmail::new(email.into())))
            .set_email_verified(Some(email_verified)),
        EmptyAdditionalClaims {},
    )
    .set_nonce(Some(Nonce::new(nonce.into())))
}

fn untrusted_claims(provider: &MockProvider, nonce: &str) -> CoreIdTokenClaims {
    native_claims(
        provider,
        &["native-ios"],
        nonce,
        "untrusted@example.test",
        "untrusted-subject",
    )
}

async fn native_login(oidc: &OidcService, provider: &MockProvider) -> Result<LoginOutcome> {
    let flow = oidc.begin_native_authorization("test").await?;
    let token = sign(
        provider,
        0,
        native_claims(
            provider,
            &["native-ios"],
            &flow.nonce,
            "mfa@example.test",
            "mfa-subject",
        ),
    );
    Ok(oidc
        .complete_native_authorization("test", &flow.flow, &token, TraceContext::default())
        .await?)
}

fn sign(provider: &MockProvider, key_index: usize, claims: CoreIdTokenClaims) -> String {
    CoreIdToken::new(
        claims,
        &provider.state.keys[key_index],
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        None,
        None,
    )
    .unwrap()
    .to_string()
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn native_id_token_login_creates_returns_and_consumes_each_flow_once() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let browser_only = MockProvider::start().await?;
    let mut browser_only_settings = browser_only.settings();
    browser_only_settings.name = "browser-only".into();
    let oidc = OidcService::discover(
        identity.clone(),
        vec![native_settings(&provider, "test"), browser_only_settings],
    )
    .await?;
    for unavailable in ["browser-only", "missing"] {
        assert!(matches!(
            oidc.begin_native_authorization(unavailable).await,
            Err(ApplicationError::NotFound)
        ));
    }

    // Android SDK shape: the web client is the audience and the native client is azp.
    let first = oidc.begin_native_authorization("test").await?;
    let token = sign(
        &provider,
        0,
        native_claims(
            &provider,
            &["test-client"],
            &first.nonce,
            "native@example.test",
            "native-subject",
        )
        .set_authorized_party(Some(ClientId::new("native-android".into()))),
    );
    let LoginOutcome::Session(created) = oidc
        .complete_native_authorization("test", &first.flow, &token, TraceContext::default())
        .await?
    else {
        anyhow::bail!("native login unexpectedly required MFA");
    };
    let account_id = identity
        .authenticate_session(&created.token)
        .await?
        .account_id();
    assert!(matches!(
        oidc.complete_native_authorization("test", &first.flow, &token, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));

    // iOS SDK shape: the native client is the only audience.
    let second = oidc.begin_native_authorization("test").await?;
    let token = sign(
        &provider,
        0,
        native_claims(
            &provider,
            &["native-ios"],
            &second.nonce,
            "native@example.test",
            "native-subject",
        ),
    );
    let LoginOutcome::Session(returned) = oidc
        .complete_native_authorization("test", &second.flow, &token, TraceContext::default())
        .await?
    else {
        anyhow::bail!("native login unexpectedly required MFA");
    };
    assert_eq!(
        identity
            .authenticate_session(&returned.token)
            .await?
            .account_id(),
        account_id
    );

    // Browser and native flows are separate and cannot complete each other.
    let redirect = oidc.begin_authorization("test", None, None).await?;
    assert!(matches!(
        oidc.complete_native_authorization(
            "test",
            &redirect.browser_binding,
            &token,
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::Unauthenticated)
    ));
    let native = oidc.begin_native_authorization("test").await?;
    assert!(matches!(
        oidc.complete_authorization(
            "test",
            &native.nonce,
            &native.flow,
            Some("public-mock-code"),
            None,
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::Unauthenticated)
    ));

    // A token signed with a rotated key refreshes provider keys once.
    provider.state.rotated.store(true, Ordering::SeqCst);
    let before = provider.state.discovery_requests.load(Ordering::SeqCst);
    let token = sign(
        &provider,
        1,
        native_claims(
            &provider,
            &["native-ios"],
            &native.nonce,
            "native@example.test",
            "native-subject",
        ),
    );
    assert!(matches!(
        oidc.complete_native_authorization("test", &native.flow, &token, TraceContext::default())
            .await?,
        LoginOutcome::Session(_)
    ));
    assert!(provider.state.discovery_requests.load(Ordering::SeqCst) > before);
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn native_id_token_login_rejects_untrusted_tokens_and_consumes_the_flow() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let second_provider = MockProvider::start().await?;
    let oidc = OidcService::discover(
        identity.clone(),
        vec![
            native_settings(&provider, "test"),
            native_settings(&second_provider, "second"),
        ],
    )
    .await?;
    let rejected: [(&str, TokenForNonce); 10] = [
        ("unconfigured audience", |provider, nonce| {
            let claims = untrusted_claims(provider, nonce)
                .set_audiences(vec![Audience::new("other-client".into())]);
            sign(provider, 0, claims)
        }),
        ("additional unconfigured audience", |provider, nonce| {
            let claims = untrusted_claims(provider, nonce)
                .set_audiences(vec![
                    Audience::new("native-ios".into()),
                    Audience::new("other-client".into()),
                ])
                .set_authorized_party(Some(ClientId::new("native-ios".into())));
            sign(provider, 0, claims)
        }),
        (
            "multiple audiences without an authorized party",
            |provider, nonce| {
                let claims = untrusted_claims(provider, nonce).set_audiences(vec![
                    Audience::new("test-client".into()),
                    Audience::new("native-ios".into()),
                ]);
                sign(provider, 0, claims)
            },
        ),
        ("unconfigured authorized party", |provider, nonce| {
            let claims = untrusted_claims(provider, nonce)
                .set_authorized_party(Some(ClientId::new("other-client".into())));
            sign(provider, 0, claims)
        }),
        ("different nonce", |provider, _| {
            sign(provider, 0, untrusted_claims(provider, "different-nonce"))
        }),
        ("missing nonce", |provider, nonce| {
            sign(
                provider,
                0,
                untrusted_claims(provider, nonce).set_nonce(None),
            )
        }),
        ("expired token", |provider, nonce| {
            let claims = untrusted_claims(provider, nonce)
                .set_expiration(chrono::Utc::now() - chrono::Duration::minutes(1));
            sign(provider, 0, claims)
        }),
        ("different issuer", |provider, nonce| {
            let claims = untrusted_claims(provider, nonce)
                .set_issuer(IssuerUrl::new("http://issuer.invalid".into()).unwrap());
            sign(provider, 0, claims)
        }),
        ("unpublished signing key", |provider, nonce| {
            sign(provider, 1, untrusted_claims(provider, nonce))
        }),
        ("malformed token", |_, _| "not-a-jwt".into()),
    ];
    for (case, token) in rejected {
        let flow = oidc.begin_native_authorization("test").await?;
        assert!(
            matches!(
                oidc.complete_native_authorization(
                    "test",
                    &flow.flow,
                    &token(&provider, &flow.nonce),
                    TraceContext::default()
                )
                .await,
                Err(ApplicationError::Unauthenticated)
            ),
            "{case} was accepted"
        );
        assert!(
            matches!(
                oidc.complete_native_authorization(
                    "test",
                    &flow.flow,
                    &sign(&provider, 0, untrusted_claims(&provider, &flow.nonce)),
                    TraceContext::default()
                )
                .await,
                Err(ApplicationError::Unauthenticated)
            ),
            "{case} did not consume the flow"
        );
    }

    // A flow sent to another configured provider stays usable at its own provider.
    let flow = oidc.begin_native_authorization("test").await?;
    let token = sign(&provider, 0, untrusted_claims(&provider, &flow.nonce));
    assert!(matches!(
        oidc.complete_native_authorization("second", &flow.flow, &token, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    assert!(matches!(
        oidc.complete_native_authorization("test", &flow.flow, &token, TraceContext::default())
            .await?,
        LoginOutcome::Session(_)
    ));

    // A new identity needs a verified email that no local account uses.
    let flow = oidc.begin_native_authorization("test").await?;
    let unverified = claims_with_email_verification(
        &provider,
        &["native-ios"],
        &flow.nonce,
        "unverified@example.test",
        "unverified-subject",
        false,
    );
    assert!(matches!(
        oidc.complete_native_authorization(
            "test",
            &flow.flow,
            &sign(&provider, 0, unverified),
            TraceContext::default()
        )
        .await,
        Err(ApplicationError::Unauthenticated)
    ));
    let (actor, _) = database.verified_account(&identity).await?;
    let email = identity.get_account(&actor).await?.email;
    let flow = oidc.begin_native_authorization("test").await?;
    let token = sign(
        &provider,
        0,
        native_claims(
            &provider,
            &["native-ios"],
            &flow.nonce,
            &email,
            "local-email-subject",
        ),
    );
    assert!(matches!(
        oidc.complete_native_authorization("test", &flow.flow, &token, TraceContext::default())
            .await,
        Err(ApplicationError::Conflict)
    ));
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn native_id_token_login_uses_the_shared_mfa_gate() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let oidc =
        OidcService::discover(identity.clone(), vec![native_settings(&provider, "test")]).await?;
    let LoginOutcome::Session(credentials) = native_login(&oidc, &provider).await? else {
        anyhow::bail!("native login unexpectedly required MFA");
    };
    let actor = identity.authenticate_session(&credentials.token).await?;
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
    let LoginOutcome::MfaRequired { challenge } = native_login(&oidc, &provider).await? else {
        anyhow::bail!("native login bypassed MFA");
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
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn native_routes_return_bearer_sessions_without_origin_or_cookies() -> Result<()> {
    use axum::http::header;
    use axum_test::TestServer;
    use kit::{bootstrap::bootstrap, http::build_router};

    let database = TestDatabase::create().await?;
    let provider = MockProvider::start().await?;
    let browser_only = MockProvider::start().await?;
    let mut settings = support::settings(&database)?;
    let mut browser_only_settings = browser_only.settings();
    browser_only_settings.name = "browser-only".into();
    settings.oidc = vec![native_settings(&provider, "test"), browser_only_settings];
    let app = bootstrap(&settings).await?;
    let api = TestServer::new(build_router(app.clone(), settings.http.clone()));

    let unavailable = api.post("/api/v1/auth/oidc/browser-only/native").await;
    unavailable.assert_status_not_found();
    assert_eq!(
        unavailable.json::<serde_json::Value>()["error"]["code"],
        "not_found"
    );

    let begin = api.post("/api/v1/auth/oidc/test/native").await;
    begin.assert_status_ok();
    assert!(begin.headers().get(header::SET_COOKIE).is_none());
    let flow = begin.json::<serde_json::Value>();
    let (flow, nonce) = (
        flow["flow"].as_str().context("flow missing")?.to_owned(),
        flow["nonce"].as_str().context("nonce missing")?.to_owned(),
    );
    let token = sign(
        &provider,
        0,
        native_claims(
            &provider,
            &["native-ios"],
            &nonce,
            "http-native@example.test",
            "http-native-subject",
        ),
    );
    api.post("/api/v1/auth/oidc/test/native/complete")
        .json(&json!({"flow": flow, "id_token": token, "transport": "cookie"}))
        .await
        .assert_status_bad_request();

    let complete = api
        .post("/api/v1/auth/oidc/test/native/complete")
        .json(&json!({"flow": flow, "id_token": token}))
        .await;
    complete.assert_status_ok();
    assert!(complete.headers().get(header::SET_COOKIE).is_none());
    let session = complete.json::<serde_json::Value>();
    assert_eq!(session["status"], "authenticated");
    let session_token = session["token"].as_str().context("token missing")?;
    api.get("/api/v1/account")
        .add_header(header::AUTHORIZATION, format!("Bearer {session_token}"))
        .await
        .assert_status_ok();

    let replay = api
        .post("/api/v1/auth/oidc/test/native/complete")
        .json(&json!({"flow": flow, "id_token": token}))
        .await;
    replay.assert_status_unauthorized();
    assert_eq!(
        replay.json::<serde_json::Value>()["error"]["code"],
        "unauthenticated"
    );
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn expired_native_flows_are_rejected_and_removed_by_cleanup() -> Result<()> {
    use application::identity::entities::oauth_native_flow;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};

    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let oidc =
        OidcService::discover(identity.clone(), vec![native_settings(&provider, "test")]).await?;
    let expired = oidc.begin_native_authorization("test").await?;
    let current = oidc.begin_native_authorization("test").await?;
    let expired_hash = auth_core::token::hash_token(&expired.flow);
    oauth_native_flow::Entity::update_many()
        .col_expr(
            oauth_native_flow::Column::ExpiresAt,
            Expr::value(chrono::Utc::now() - chrono::Duration::minutes(1)),
        )
        .filter(oauth_native_flow::Column::FlowHash.eq(expired_hash.clone()))
        .exec(&database.connection)
        .await?;
    let token = sign(&provider, 0, untrusted_claims(&provider, &expired.nonce));
    assert!(matches!(
        oidc.complete_native_authorization("test", &expired.flow, &token, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));

    assert!(identity.cleanup_expired_authentication().await? >= 1);
    let remaining = oauth_native_flow::Entity::find()
        .all(&database.connection)
        .await?;
    assert_eq!(remaining.len(), 1);
    assert_ne!(remaining[0].flow_hash, expired_hash);
    let token = sign(&provider, 0, untrusted_claims(&provider, &current.nonce));
    assert!(matches!(
        oidc.complete_native_authorization("test", &current.flow, &token, TraceContext::default())
            .await?,
        LoginOutcome::Session(_)
    ));
    database.finish().await
}
