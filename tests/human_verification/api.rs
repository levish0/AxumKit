use super::*;
use application::identity::entities::account;
use axum::http::header;
use axum_test::TestServer;
use kit::{bootstrap::bootstrap, http::build_router};
use sea_orm::{ActiveModelTrait, Set};
use test_support::TestDatabase;

/// Creates a verified account whose password is generated at runtime.
async fn verified_account(database: &TestDatabase) -> Result<(String, String)> {
    let email = format!("{}@example.test", uuid::Uuid::now_v7());
    let password = auth_core::token::generate_secure_token();
    let hashed_password = password.clone();
    let password_hash =
        tokio::task::spawn_blocking(move || auth_core::password::hash(&hashed_password)).await??;
    account::ActiveModel {
        id: Set(uuid::Uuid::now_v7()),
        email: Set(email.clone()),
        password_hash: Set(Some(password_hash)),
        verified: Set(true),
        ..Default::default()
    }
    .insert(&database.connection)
    .await?;
    Ok((email, password))
}

fn assert_error(response: &axum_test::TestResponse, status: StatusCode, code: &str) {
    response.assert_status(status);
    assert_eq!(response.json::<Value>()["error"]["code"], code);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn tunnel_and_ssr_identity_share_limits_and_reach_human_verification() -> Result<()> {
    let database = TestDatabase::create().await?;
    let provider = FakeSiteverify::start().await?;
    let secret = auth_core::token::generate_secure_token();
    let mut settings = support::settings(&database)?;
    settings.http.internal_proxy_secret = Some(SecretString::new(secret.clone()));
    settings.identity.authentication_client_limit_per_minute = 2;
    settings.human_verification = Some(provider.settings());
    let first_app = bootstrap(&settings).await?;
    let second_app = bootstrap(&settings).await?;
    let first = TestServer::new(build_router(first_app.clone(), settings.http.clone()));
    let second = TestServer::new(build_router(second_app.clone(), settings.http));
    let body = json!({
        "email": "tunnel-identity@example.test",
        "human_verification_token": "valid:email_verification"
    });

    for (server, presented, original, expected_ip, status) in [
        (
            &first,
            None,
            "198.51.100.7",
            "192.0.2.10",
            StatusCode::ACCEPTED,
        ),
        (
            &second,
            Some("incorrect"),
            "198.51.100.8",
            "192.0.2.10",
            StatusCode::ACCEPTED,
        ),
        (
            &first,
            None,
            "198.51.100.9",
            "192.0.2.10",
            StatusCode::TOO_MANY_REQUESTS,
        ),
        (
            &first,
            Some(secret.as_str()),
            "198.51.100.7",
            "198.51.100.7",
            StatusCode::ACCEPTED,
        ),
        (
            &second,
            Some(secret.as_str()),
            "198.51.100.7",
            "198.51.100.7",
            StatusCode::ACCEPTED,
        ),
        (
            &first,
            Some(secret.as_str()),
            "198.51.100.7",
            "198.51.100.7",
            StatusCode::TOO_MANY_REQUESTS,
        ),
    ] {
        let mut request = server
            .post("/api/v1/auth/verification")
            .add_header("cf-connecting-ip", "192.0.2.10")
            .add_header("x-real-client-ip", original)
            .add_header("x-forwarded-for", original)
            .json(&body);
        if let Some(presented) = presented {
            request = request.add_header("x-internal-secret", presented);
        }
        let response = request.await;
        response.assert_status(status);
        let calls = provider.take_requests();
        if status == StatusCode::ACCEPTED {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["remoteip"], expected_ip);
        } else {
            assert!(calls.is_empty());
            assert_eq!(response.json::<Value>()["error"]["code"], "rate_limited");
            assert!(response.headers().contains_key(header::RETRY_AFTER));
        }
        assert!(!response.text().contains(&secret));
    }
    second
        .post("/api/v1/auth/verification")
        .add_header("cf-connecting-ip", "203.0.113.20")
        .json(&body)
        .await
        .assert_status(StatusCode::ACCEPTED);
    assert_eq!(provider.take_requests()[0]["remoteip"], "203.0.113.20");
    first
        .post("/api/v1/auth/verification")
        .add_header("cf-connecting-ip", "invalid")
        .json(&body)
        .await
        .assert_status(StatusCode::BAD_REQUEST);
    assert!(provider.take_requests().is_empty());
    first_app.close().await?;
    second_app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn configured_turnstile_protects_public_identity_operations() -> Result<()> {
    let database = TestDatabase::create().await?;
    let provider = FakeSiteverify::start().await?;
    let mut settings = support::settings(&database)?;
    settings.human_verification = Some(provider.settings());
    let app = bootstrap(&settings).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let (email, password) = verified_account(&database).await?;
    let signup_password = auth_core::token::generate_secure_token();

    let response = server
        .post("/api/v1/auth/signup")
        .json(&json!({"email": "turnstile@example.test", "password": signup_password}))
        .await;
    assert_error(
        &response,
        StatusCode::FORBIDDEN,
        "human_verification_failed",
    );
    let response = server
        .post("/api/v1/auth/signup")
        .json(
            &json!({"email": "turnstile@example.test", "password": signup_password,
            "human_verification_token": "valid:login"}),
        )
        .await;
    assert_error(
        &response,
        StatusCode::FORBIDDEN,
        "human_verification_failed",
    );
    assert_eq!(provider.take_requests().len(), 1);
    server
        .post("/api/v1/auth/signup")
        .json(
            &json!({"email": "turnstile@example.test", "password": signup_password,
            "human_verification_token": "valid:signup"}),
        )
        .await
        .assert_status(StatusCode::ACCEPTED);
    let overlong = "x".repeat(2049);
    let response = server
        .post("/api/v1/auth/signup")
        .json(
            &json!({"email": "turnstile@example.test", "password": signup_password,
            "human_verification_token": overlong}),
        )
        .await;
    assert_error(&response, StatusCode::BAD_REQUEST, "invalid_input");
    assert!(!response.text().contains(&overlong));

    for (path, action) in [
        ("/api/v1/auth/verification", "email_verification"),
        ("/api/v1/auth/password/reset", "password_reset"),
    ] {
        let response = server.post(path).json(&json!({"email": email})).await;
        assert_error(
            &response,
            StatusCode::FORBIDDEN,
            "human_verification_failed",
        );
        let response = server
            .post(path)
            .json(&json!({"email": email, "human_verification_token": "valid:signup"}))
            .await;
        assert_error(
            &response,
            StatusCode::FORBIDDEN,
            "human_verification_failed",
        );
        server
            .post(path)
            .json(&json!({"email": email, "human_verification_token": format!("valid:{action}")}))
            .await
            .assert_status(StatusCode::ACCEPTED);
    }

    let response = server
        .post("/api/v1/auth/login")
        .json(&json!({"email": email, "password": password, "transport": "bearer"}))
        .await;
    assert_error(
        &response,
        StatusCode::FORBIDDEN,
        "human_verification_failed",
    );
    let response = server
        .post("/api/v1/auth/login")
        .json(
            &json!({"email": email, "password": password, "transport": "bearer",
            "human_verification_token": "other-host"}),
        )
        .await;
    assert_error(
        &response,
        StatusCode::FORBIDDEN,
        "human_verification_failed",
    );
    let response = server
        .post("/api/v1/auth/login")
        .json(
            &json!({"email": email, "password": password, "transport": "bearer",
            "human_verification_token": "unavailable"}),
        )
        .await;
    assert_error(
        &response,
        StatusCode::SERVICE_UNAVAILABLE,
        "human_verification_unavailable",
    );
    let response = server
        .post("/api/v1/auth/login")
        .json(
            &json!({"email": email, "password": password, "transport": "bearer",
            "human_verification_token": "valid:login"}),
        )
        .await;
    response.assert_status_ok();
    assert_eq!(response.json::<Value>()["status"], "authenticated");

    let requests = provider.take_requests();
    assert!(
        requests
            .iter()
            .all(|request| request["secret"] == provider.secret()
                && request.get("remoteip").is_none())
    );
    for request in &requests {
        let body = request.to_string();
        assert!(!body.contains(&password) && !body.contains(&signup_password));
    }
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn unconfigured_turnstile_keeps_public_identity_operations_unchanged() -> Result<()> {
    let database = TestDatabase::create().await?;
    let settings = support::settings(&database)?;
    assert!(settings.human_verification.is_none());
    let app = bootstrap(&settings).await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let (email, password) = verified_account(&database).await?;
    let signup_password = auth_core::token::generate_secure_token();

    for token in [None, Some("ignored-token")] {
        let mut signup = json!({"email": "optional@example.test", "password": signup_password});
        let mut reset = json!({"email": email});
        let mut login = json!({"email": email, "password": password, "transport": "bearer"});
        if let Some(token) = token {
            for body in [&mut signup, &mut reset, &mut login] {
                body["human_verification_token"] = json!(token);
            }
        }
        server
            .post("/api/v1/auth/signup")
            .json(&signup)
            .await
            .assert_status(StatusCode::ACCEPTED);
        for path in ["/api/v1/auth/verification", "/api/v1/auth/password/reset"] {
            server
                .post(path)
                .json(&reset)
                .await
                .assert_status(StatusCode::ACCEPTED);
        }
        let response = server.post("/api/v1/auth/login").json(&login).await;
        response.assert_status_ok();
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }
    app.close().await?;
    database.finish().await
}
