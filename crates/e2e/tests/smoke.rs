//! Smoke tests proving the e2e stack and harness work end to end.
//! Run via `just e2e` (which brings up docker-compose.test.yml first).

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
async fn signup_verify_login_flow() {
    // Exercises the full path: signup → worker sends email → mailpit → token →
    // verify-email sets the session cookie → an authenticated endpoint succeeds.
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // The session cookie from verification should now authorize a "me" read.
    let resp = client.get("/v0/user/me").await;
    assert!(
        resp.status().is_success(),
        "authenticated profile read should succeed after verification; got {}",
        resp.status()
    );

    // A fresh, cookie-less client must be rejected on the same endpoint.
    let anon = TestClient::new();
    let resp = anon.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "anonymous profile read must be unauthorized"
    );

    // Logging in again from a fresh browser: credentials are accepted but the
    // unknown device is challenged by email (202); completing the challenge
    // issues the session (204 + cookies).
    let before_device_mail = e2e::latest_message_id(&user.email).await;
    let login = TestClient::new();
    let resp = login
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password, "remember_me": false }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "unknown-device login should be held for verification"
    );
    let body = e2e::wait_for_new_message_body(&user.email, before_device_mail.as_deref()).await;
    let device_token = e2e::extract_token(&body).expect("device verification token");
    let resp = login
        .post_json("/v0/auth/device/verify", &json!({ "token": device_token }))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "device verify should issue the session"
    );
    let resp = login.get("/v0/user/me").await;
    assert!(resp.status().is_success(), "session after device verify");
}

#[tokio::test]
async fn duplicate_handle_is_rejected() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Re-using the same handle for a new signup must conflict.
    let resp = client
        .post_json(
            "/v0/auth/signup",
            &json!({
                "email": format!("other-{}@test.invalid", e2e::unique()),
                "handle": user.handle,
                "display_name": "Dup",
                "password": "e2e-password-123",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn login_with_wrong_password_is_unauthorized() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    let attacker = TestClient::new();
    let resp = attacker
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": "wrong-password", "remember_me": false }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn profile_image_upload_is_processed_to_webp() {
    let anon = TestClient::new();
    let part = reqwest::multipart::Part::bytes(e2e::test_png([1, 2, 3, 255]))
        .file_name("avatar.png")
        .mime_str("image/png")
        .expect("valid mime type");
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = anon.post_multipart("/v0/user/me/profile-image", form).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let client = TestClient::new();
    client.signup_and_login().await;

    let part = reqwest::multipart::Part::bytes(e2e::test_png([200, 30, 30, 255]))
        .file_name("avatar.png")
        .mime_str("image/png")
        .expect("valid mime type");
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = client
        .post_multipart("/v0/user/me/profile-image", form)
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;

    let image_url = body["image_url"]
        .as_str()
        .expect("image_url should be a string");
    assert!(
        image_url.ends_with(".webp"),
        "profile image should be stored as webp; got {image_url}"
    );
}
