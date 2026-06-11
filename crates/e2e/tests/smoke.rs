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
    client.signup_and_verify().await;

    let part = reqwest::multipart::Part::bytes(e2e::test_png([200, 30, 30, 255]))
        .file_name("avatar.png")
        .mime_str("image/png")
        .expect("valid mime type");
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = client.post_multipart("/v0/user/me/profile-image", form).await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;

    let image_url = body["image_url"]
        .as_str()
        .expect("image_url should be a string");
    assert!(
        image_url.ends_with(".webp"),
        "profile image should be stored as webp; got {image_url}"
    );
}
