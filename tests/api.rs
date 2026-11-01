mod support;
use axum::http::{StatusCode, header};
use axum_test::TestServer;
use kit::{bootstrap::bootstrap, http::build_router};
use serde_json::{Value, json};
use test_support::{TEST_PASSWORD, TestDatabase};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn recovery_code_endpoint_replaces_codes_once_and_clears_the_session() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let (actor, original_session) = database.verified_account(&app.identity).await?;
    let email = app.identity.get_account(&actor).await?.email;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let authorization = format!("Bearer {}", original_session.token);
    let enrollment = server
        .post("/api/v1/account/totp")
        .add_header(header::AUTHORIZATION, &authorization)
        .await;
    enrollment.assert_status_ok();
    let enrollment: Value = enrollment.json();
    let authenticator = totp_rs::Totp::from_url(enrollment["authenticator_url"].as_str().unwrap())?;
    let response = server.post("/api/v1/account/totp/confirm").add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"challenge":enrollment["challenge"], "code":authenticator.generate_current().to_string()})).await;
    response.assert_status_ok();
    let codes: Value = response.json();
    let login = server
        .post("/api/v1/auth/login")
        .json(&json!({"email":email,"password":TEST_PASSWORD,"transport":"bearer"}))
        .await;
    login.assert_status_ok();
    let login: Value = login.json();
    let authenticated = server
        .post("/api/v1/auth/mfa")
        .json(&json!({"challenge":login["challenge"],
        "proof":{"kind":"recovery_code","code":codes["recovery_codes"][0]},"transport":"bearer"}))
        .await;
    authenticated.assert_status_ok();
    let authenticated: Value = authenticated.json();
    let authorization = format!("Bearer {}", authenticated["token"].as_str().unwrap());
    let response = server
        .post("/api/v1/account/recovery-codes")
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"kind":"recovery_code","code":codes["recovery_codes"][1]}))
        .await;
    response.assert_status_ok();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        response.headers()[header::SET_COOKIE]
            .to_str()?
            .contains("Max-Age=0")
    );
    let replacements: Value = response.json();
    assert_eq!(replacements["recovery_codes"].as_array().unwrap().len(), 10);
    server
        .get("/api/v1/account")
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_unauthorized();
    let login: Value = server
        .post("/api/v1/auth/login")
        .json(&json!({"email":email,"password":TEST_PASSWORD,"transport":"bearer"}))
        .await
        .json();
    server
        .post("/api/v1/auth/mfa")
        .json(&json!({"challenge":login["challenge"],
        "proof":{"kind":"recovery_code","code":codes["recovery_codes"][2]},"transport":"bearer"}))
        .await
        .assert_status_unauthorized();
    server.post("/api/v1/auth/mfa").json(&json!({"challenge":login["challenge"],
        "proof":{"kind":"recovery_code","code":replacements["recovery_codes"][0]},"transport":"bearer"})).await.assert_status_ok();
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn dto_validation_reuses_domain_rules_without_exposing_submitted_secrets()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let overlong_password = "비".repeat(342);
    let response = server
        .post("/api/v1/auth/signup")
        .json(&json!({"email":"unicode@example.test", "password":overlong_password}))
        .await;
    response.assert_status_bad_request();
    assert!(!response.text().contains(&overlong_password));
    assert!(matches!(
        app.identity
            .register_account(
                "unicode@example.test",
                overlong_password,
                jobs::TraceContext::default()
            )
            .await,
        Err(application::ApplicationError::InvalidInput(_))
    ));
    server
        .post("/api/v1/auth/signup")
        .json(&json!({"email":"unicode@example.test", "password":"비".repeat(15)}))
        .await
        .assert_status(StatusCode::ACCEPTED);
    server.post("/api/v1/auth/mfa")
        .json(&json!({"challenge":"invalid", "proof":{"kind":"totp", "code":""}, "transport":"bearer"})).await
        .assert_status_bad_request();
    let (actor, credentials) = database.verified_account(&app.identity).await?;
    let email = app.identity.get_account(&actor).await?.email;
    server
        .post("/api/v1/auth/login")
        .json(&json!({"email":email, "password":"wrong", "transport":"bearer"}))
        .await
        .assert_status_unauthorized();
    let authorization = format!("Bearer {}", credentials.token);
    server
        .post("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":"  "}))
        .await
        .assert_status_bad_request();
    let response = server
        .post("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":"  Valid name  "}))
        .await;
    response.assert_status(StatusCode::CREATED);
    assert_eq!(response.json::<Value>()["name"], "Valid name");
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn organizations_are_scoped_for_detail_list_and_updates() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let (owner, owner_credentials) = database.verified_account(&app.identity).await?;
    let (_, other_credentials) = database.verified_account(&app.identity).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let authorization = format!("Bearer {}", owner_credentials.token);
    let other_authorization = format!("Bearer {}", other_credentials.token);
    let created = server
        .post("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":"First organization"}))
        .await;
    created.assert_status(StatusCode::CREATED);
    let organization: Value = created.json();
    assert_eq!(organization["owner_id"], owner.account_id().to_string());
    let organization_url = format!(
        "/api/v1/organizations/{}",
        organization["id"].as_str().unwrap()
    );
    server
        .get(&organization_url)
        .add_header(header::AUTHORIZATION, &other_authorization)
        .await
        .assert_status_not_found();
    let hidden: Value = server
        .get("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &other_authorization)
        .await
        .json();
    assert_eq!(hidden["items"], json!([]));
    server
        .put(&organization_url)
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":"Updated", "expected_version":1}))
        .await
        .assert_status_ok();
    server
        .put(&organization_url)
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":"Stale update", "expected_version":1}))
        .await
        .assert_status_conflict();
    server
        .put(&organization_url)
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"name":null,"expected_version":2}))
        .await
        .assert_status_bad_request();
    server
        .get("/api/v1/organizations?limit=0")
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_bad_request();
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn cookie_login_requires_origin_and_protected_writes_reject_csrf() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let (actor, _) = database.verified_account(&app.identity).await?;
    let email = app.identity.get_account(&actor).await?.email;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let login = json!({"email":email, "password":TEST_PASSWORD});
    server
        .post("/api/v1/auth/login")
        .json(&login)
        .await
        .assert_status_forbidden();
    let response = server
        .post("/api/v1/auth/login")
        .add_header(header::ORIGIN, "http://localhost:3000")
        .json(&login)
        .await;
    response.assert_status_ok();
    let value: Value = response.json();
    assert!(value.get("token").is_none());
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()?;
    assert!(
        cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax") && cookie.contains("Path=/")
    );
    let cookie = cookie.split(';').next().unwrap();
    server
        .post("/api/v1/organizations")
        .add_header(header::COOKIE, cookie)
        .json(&json!({"name":"Rejected"}))
        .await
        .assert_status_forbidden();
    server
        .post("/api/v1/organizations")
        .add_header(header::COOKIE, cookie)
        .add_header(header::ORIGIN, "https://evil.example")
        .json(&json!({"name":"Rejected"}))
        .await
        .assert_status_forbidden();
    server
        .post("/api/v1/organizations")
        .add_header(header::COOKIE, cookie)
        .add_header(header::ORIGIN, "http://localhost:3000")
        .json(&json!({"name":"Allowed"}))
        .await
        .assert_status(StatusCode::CREATED);
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn production_router_normalizes_errors_and_replaces_untrusted_request_ids()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    for path in ["/missing", "/api/v1/account"] {
        let response = server
            .get(path)
            .add_header("x-request-id", "untrusted-value")
            .await;
        let body: Value = response.json();
        let request_id = response.headers().get("x-request-id").unwrap().to_str()?;
        assert_eq!(body["request_id"], request_id);
        assert_eq!(uuid::Uuid::parse_str(request_id)?.get_version_num(), 7);
        assert!(body["error"]["code"].is_string());
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    }
    server
        .post("/api/v1/auth/signup")
        .json(&json!({"email":"x@example.test","password":"short", "unexpected":true}))
        .await
        .assert_status_bad_request();
    server
        .post("/api/v1/auth/signup")
        .json(&json!({"email":"x@example.test","password":"x".repeat(20000)}))
        .await
        .assert_status_payload_too_large();
    server
        .get("/api/v1/auth/signup")
        .await
        .assert_status(StatusCode::METHOD_NOT_ALLOWED);
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn two_applications_share_sessions_but_keep_http_configuration_separate() -> anyhow::Result<()>
{
    let database = TestDatabase::create().await?;
    let first_settings = support::settings(&database)?;
    let first = bootstrap(&first_settings).await?;
    let mut second_settings = support::settings(&database)?;
    second_settings.http.allowed_origins = vec!["http://localhost:4000".into()];
    let second = bootstrap(&second_settings).await?;
    let (_, credentials) = database.verified_account(&first.identity).await?;
    let first_server = TestServer::new(build_router(first.clone(), first_settings.http));
    let second_server = TestServer::new(build_router(second.clone(), second_settings.http));
    let authorization = format!("Bearer {}", credentials.token);
    second_server
        .get("/api/v1/account")
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_ok();
    first_server
        .post("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &authorization)
        .add_header(header::ORIGIN, "http://localhost:4000")
        .json(&json!({"name":"Rejected"}))
        .await
        .assert_status_forbidden();
    second_server
        .post("/api/v1/organizations")
        .add_header(header::AUTHORIZATION, &authorization)
        .add_header(header::ORIGIN, "http://localhost:4000")
        .json(&json!({"name":"Allowed"}))
        .await
        .assert_status(StatusCode::CREATED);
    first_server
        .delete(&format!("/api/v1/sessions/{}", credentials.session_id))
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_no_content();
    second_server
        .get("/api/v1/account")
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_unauthorized();
    first.close().await?;
    second.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn storage_failure_is_not_treated_as_anonymous() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    let app = bootstrap(&settings).await?;
    let (_, credentials) = database.verified_account(&app.identity).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    app.close().await?;
    server
        .get("/api/v1/account")
        .add_header(
            header::AUTHORIZATION,
            format!("Bearer {}", credentials.token),
        )
        .await
        .assert_status_service_unavailable();
    server.get("/health/live").await.assert_status_no_content();
    server
        .get("/health/ready")
        .await
        .assert_status_service_unavailable();
    database.finish().await
}
