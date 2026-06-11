use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn health_check_returns_no_content() {
    let client = TestClient::new();
    let resp = client.get("/health-check").await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn signup_verify_and_login_flow() {
    let client = TestClient::new();
    let user = client.signup_and_verify().await;

    let login = TestClient::new();
    let resp = login
        .post_json(
            "/v0/auth/login",
            &json!({
                "email": user.email,
                "password": user.password,
                "remember_me": false,
            }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "login should succeed"
    );

    let resp = login.get("/v0/user/me").await;
    assert!(
        resp.status().is_success(),
        "authenticated profile read should succeed; got {}",
        resp.status()
    );
}

#[tokio::test]
async fn duplicate_handle_is_rejected() {
    let client = TestClient::new();
    let user = client.signup_and_verify().await;

    let resp = client
        .post_json(
            "/v0/auth/signup",
            &json!({
                "email": format!("other-{}@test.invalid", e2e::unique()),
                "handle": user.handle,
                "display_name": "Duplicate",
                "password": "e2e-pass-123",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
