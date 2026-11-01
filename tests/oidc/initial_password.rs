use super::*;
use application::identity::{
    IdentityService, SessionCredentials,
    entities::{account, audit_event, mail_delivery, session},
};
use mail::{AuthenticationMail, MailPurpose};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set,
};
use uuid::Uuid;

/// Creates a valid password at runtime, so tests keep credentials out of source literals.
fn generated_password() -> String {
    auth_core::token::generate_secure_token()
}

/// Creates a password below the 15-character minimum.
fn generated_short_password() -> String {
    generated_password()[..9].to_owned()
}

async fn oidc_session(
    oidc: &OidcService,
    provider: &MockProvider,
    email: &str,
    subject: &str,
) -> Result<SessionCredentials> {
    let redirect = oidc.begin_authorization("test", None, None).await?;
    let (state, code) = provider.authorize(&redirect, email, subject, false, false)?;
    let CallbackOutcome::Login(LoginOutcome::Session(credentials)) = oidc
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
        anyhow::bail!("OIDC login did not issue a session");
    };
    Ok(credentials)
}

async fn stored_account(database: &TestDatabase, account_id: Uuid) -> Result<account::Model> {
    account::Entity::find_by_id(account_id)
        .one(&database.connection)
        .await?
        .context("account missing")
}

async fn password_change_notifications(database: &TestDatabase, account_id: Uuid) -> Result<usize> {
    let settings = test_support::identity_settings()?;
    let mut count = 0;
    for delivery in mail_delivery::Entity::find()
        .filter(mail_delivery::Column::AccountId.eq(account_id))
        .all(&database.connection)
        .await?
    {
        let plaintext = settings.secret_keys.decrypt(
            &format!("authentication mail v1:{}", delivery.id),
            delivery
                .ciphertext
                .as_deref()
                .context("ciphertext missing")?,
        )?;
        let mail: AuthenticationMail = serde_json::from_slice(&plaintext)?;
        if matches!(mail.purpose, MailPurpose::PasswordChanged) {
            ensure!(
                delivery.challenge_hash.is_none(),
                "notification has a token"
            );
            count += 1;
        }
    }
    Ok(count)
}

async fn initial_password_audit_events(database: &TestDatabase, account_id: Uuid) -> Result<u64> {
    Ok(audit_event::Entity::find()
        .filter(audit_event::Column::AccountId.eq(account_id))
        .filter(audit_event::Column::Event.eq("initial_password_set"))
        .count(&database.connection)
        .await?)
}

async fn login_with_password(
    identity: &IdentityService,
    email: &str,
    password: &str,
) -> Result<SessionCredentials, ApplicationError> {
    match identity
        .login_with_password(email, password.into(), TraceContext::default())
        .await?
    {
        LoginOutcome::Session(credentials) => Ok(credentials),
        LoginOutcome::MfaRequired { .. } => Err(ApplicationError::Forbidden),
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn oidc_only_account_sets_one_initial_password_and_revokes_credentials() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let provider = MockProvider::start().await?;
    let oidc = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let email = "initial-password@example.test";
    let initial_password = generated_password();
    let credentials = oidc_session(&oidc, &provider, email, "initial-subject").await?;
    let other_credentials = oidc_session(&oidc, &provider, email, "initial-subject").await?;
    let actor = identity.authenticate_session(&credentials.token).await?;
    let other_actor = identity
        .authenticate_session(&other_credentials.token)
        .await?;
    let account_id = actor.account_id();
    assert!(!identity.get_account(&actor).await?.has_password);
    assert!(matches!(
        login_with_password(&identity, email, &initial_password).await,
        Err(ApplicationError::Unauthenticated)
    ));

    assert!(matches!(
        identity
            .set_initial_password(&actor, generated_short_password(), TraceContext::default())
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    let mut stale_session = session::Entity::find_by_id(other_actor.session_id())
        .one(&database.connection)
        .await?
        .context("session missing")?
        .into_active_model();
    stale_session.authenticated_at = Set(chrono::Utc::now() - chrono::Duration::minutes(10));
    stale_session.update(&database.connection).await?;
    assert!(matches!(
        identity
            .set_initial_password(
                &other_actor,
                initial_password.clone(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Forbidden)
    ));
    let before = stored_account(&database, account_id).await?;
    assert!(before.password_hash.is_none());
    assert_eq!(
        password_change_notifications(&database, account_id).await?,
        0
    );

    identity
        .set_initial_password(&actor, initial_password.clone(), TraceContext::default())
        .await?;
    let after = stored_account(&database, account_id).await?;
    assert!(after.password_hash.is_some());
    assert_eq!(after.security_version, before.security_version + 1);
    for token in [&credentials.token, &other_credentials.token] {
        assert!(matches!(
            identity.authenticate_session(token).await,
            Err(ApplicationError::Unauthenticated)
        ));
    }
    assert_eq!(
        password_change_notifications(&database, account_id).await?,
        1
    );
    assert_eq!(
        initial_password_audit_events(&database, account_id).await?,
        1
    );

    let password_session = login_with_password(&identity, email, &initial_password).await?;
    let password_actor = identity
        .authenticate_session(&password_session.token)
        .await?;
    assert!(identity.get_account(&password_actor).await?.has_password);
    assert!(matches!(
        identity
            .set_initial_password(
                &password_actor,
                generated_password(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Conflict)
    ));
    let unchanged = stored_account(&database, account_id).await?;
    assert_eq!(unchanged.password_hash, after.password_hash);
    assert_eq!(unchanged.security_version, after.security_version);
    assert_eq!(
        password_change_notifications(&database, account_id).await?,
        1
    );
    assert_eq!(
        initial_password_audit_events(&database, account_id).await?,
        1
    );
    identity
        .authenticate_session(&password_session.token)
        .await?;
    oidc_session(&oidc, &provider, email, "initial-subject").await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn existing_and_concurrent_passwords_are_never_replaced_by_initial_password() -> Result<()> {
    let database = TestDatabase::create().await?;
    let identity = database.identity_service().await?;
    let (password_actor, password_credentials) = database.verified_account(&identity).await?;
    let password_email = identity.get_account(&password_actor).await?.email;
    let existing = stored_account(&database, password_actor.account_id()).await?;
    let rejected_password = generated_password();
    assert!(matches!(
        identity
            .set_initial_password(
                &password_actor,
                rejected_password.clone(),
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Conflict)
    ));
    let unchanged = stored_account(&database, password_actor.account_id()).await?;
    assert_eq!(unchanged.password_hash, existing.password_hash);
    assert_eq!(unchanged.security_version, existing.security_version);
    identity
        .authenticate_session(&password_credentials.token)
        .await?;
    assert!(matches!(
        login_with_password(&identity, &password_email, &rejected_password).await,
        Err(ApplicationError::Unauthenticated)
    ));

    let provider = MockProvider::start().await?;
    let oidc = OidcService::discover(identity.clone(), vec![provider.settings()]).await?;
    let email = "concurrent-initial@example.test";
    let credentials = oidc_session(&oidc, &provider, email, "concurrent-subject").await?;
    let actor = identity.authenticate_session(&credentials.token).await?;
    let before = stored_account(&database, actor.account_id()).await?;
    let candidates = [generated_password(), generated_password()];
    let attempts = candidates.clone().map(|password| {
        let identity = identity.clone();
        let actor = actor.clone();
        tokio::spawn(async move {
            identity
                .set_initial_password(&actor, password, TraceContext::default())
                .await
        })
    });
    let mut succeeded = Vec::new();
    for (password, attempt) in candidates.iter().zip(attempts) {
        match attempt.await? {
            Ok(()) => succeeded.push(password),
            Err(ApplicationError::Conflict | ApplicationError::Unauthenticated) => {}
            Err(error) => return Err(error.into()),
        }
    }
    assert_eq!(succeeded.len(), 1);
    let after = stored_account(&database, actor.account_id()).await?;
    assert_eq!(after.security_version, before.security_version + 1);
    assert_eq!(
        password_change_notifications(&database, actor.account_id()).await?,
        1
    );
    assert_eq!(
        initial_password_audit_events(&database, actor.account_id()).await?,
        1
    );
    for password in &candidates {
        let login = login_with_password(&identity, email, password).await;
        if password == succeeded[0] {
            login?;
        } else {
            assert!(matches!(login, Err(ApplicationError::Unauthenticated)));
        }
    }
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn initial_password_endpoint_validates_input_and_clears_the_session() -> Result<()> {
    use axum::http::header;
    use axum_test::TestServer;
    use kit::{bootstrap::bootstrap, http::build_router};

    let database = TestDatabase::create().await?;
    let provider = MockProvider::start().await?;
    let mut settings = support::settings(&database)?;
    settings.oidc = vec![provider.settings()];
    let app = bootstrap(&settings).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let email = "initial-endpoint@example.test";
    let initial_password = generated_password();

    let response = server
        .post("/api/v1/auth/oidc/test")
        .add_header(header::ORIGIN, "http://localhost:3000")
        .await;
    response.assert_status_ok();
    let binding_cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .context("OIDC binding cookie missing")?
        .to_owned();
    let redirect = AuthorizationRedirect {
        url: response.json::<serde_json::Value>()["authorization_url"]
            .as_str()
            .context("authorization URL missing")?
            .parse()?,
        browser_binding: binding_cookie
            .split_once('=')
            .context("OIDC binding missing")?
            .1
            .into(),
    };
    let (state, code) = provider.authorize(&redirect, email, "endpoint-subject", false, false)?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("state", &state)
        .append_pair("code", &code)
        .finish();
    let callback = server
        .get(&format!("/api/v1/auth/oidc/test/callback?{query}"))
        .add_header(header::COOKIE, &binding_cookie)
        .await;
    callback.assert_status_ok();
    let session_cookie = callback
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .find(|header| header.starts_with("session="))
        .context("session cookie missing")?
        .split(';')
        .next()
        .context("session cookie value missing")?
        .to_owned();

    let account = server
        .get("/api/v1/account")
        .add_header(header::COOKIE, &session_cookie)
        .await;
    account.assert_status_ok();
    assert_eq!(account.json::<serde_json::Value>()["has_password"], false);
    server
        .post("/api/v1/account/password/initial")
        .json(&json!({"new_password": initial_password}))
        .await
        .assert_status_unauthorized();
    let short_password = generated_short_password();
    let response = server
        .post("/api/v1/account/password/initial")
        .add_header(header::COOKIE, &session_cookie)
        .add_header(header::ORIGIN, "http://localhost:3000")
        .json(&json!({"new_password": short_password}))
        .await;
    response.assert_status_bad_request();
    assert!(!response.text().contains(&short_password));
    server
        .post("/api/v1/account/password/initial")
        .add_header(header::COOKIE, &session_cookie)
        .add_header(header::ORIGIN, "http://localhost:3000")
        .json(&json!({"new_password": initial_password, "current_password": generated_password()}))
        .await
        .assert_status_bad_request();

    let response = server
        .post("/api/v1/account/password/initial")
        .add_header(header::COOKIE, &session_cookie)
        .add_header(header::ORIGIN, "http://localhost:3000")
        .json(&json!({"new_password": initial_password}))
        .await;
    response.assert_status(StatusCode::NO_CONTENT);
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .contains("Max-Age=0")
    );
    server
        .get("/api/v1/account")
        .add_header(header::COOKIE, &session_cookie)
        .await
        .assert_status_unauthorized();

    let login = server
        .post("/api/v1/auth/login")
        .json(&json!({"email": email, "password": initial_password, "transport": "bearer"}))
        .await;
    login.assert_status_ok();
    let authorization = format!(
        "Bearer {}",
        login.json::<serde_json::Value>()["token"]
            .as_str()
            .context("bearer token missing")?
    );
    let account = server
        .get("/api/v1/account")
        .add_header(header::AUTHORIZATION, &authorization)
        .await;
    account.assert_status_ok();
    assert_eq!(account.json::<serde_json::Value>()["has_password"], true);
    let conflict = server
        .post("/api/v1/account/password/initial")
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"new_password": generated_password()}))
        .await;
    conflict.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        conflict.json::<serde_json::Value>()["error"]["code"],
        "conflict"
    );
    app.close().await?;
    database.finish().await
}
