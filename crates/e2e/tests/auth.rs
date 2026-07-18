//! Auth/session e2e tests beyond the smoke flow. Run via `just e2e`.
//!
//! Policy references:
//! - Password reset: one-shot token by email; completing the reset invalidates ALL
//!   existing sessions and the old password.
//! - Sessions: `/v0/auth/sessions` lists only the caller's sessions (exactly one
//!   flagged `is_current`); revoking by `management_id` only works on one's own
//!   sessions — a foreign id yields 404 (existence is hidden), never a cross-user kill.
//! - Change password: requires the current password; a wrong current password is
//!   rejected.
//! - Logout deletes the current server-side session and clears the browser cookie.
//! - Resending verification is enumeration-safe (204 for unknown emails) and sends a
//!   new message for a still-pending email/password signup.

use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::{Value, json};

#[tokio::test]
async fn password_reset_invalidates_sessions_and_old_password() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Remember the newest existing email (the signup verification) so the wait below
    // can't mistake it for the reset email.
    let before = e2e::latest_message_id(&user.email).await;

    let resp = client
        .post_json("/v0/auth/forgot-password", &json!({ "email": user.email }))
        .await;
    assert!(
        resp.status().is_success(),
        "forgot-password failed: {}",
        resp.status()
    );

    let body = e2e::wait_for_new_message_body(&user.email, before.as_deref()).await;
    let token = e2e::extract_token(&body).expect("reset token in email");

    let new_password = "e2e-new-password-456";
    let resp = client
        .post_json(
            "/v0/auth/reset-password",
            &json!({ "token": token, "new_password": new_password }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "reset-password failed: {}",
        resp.status()
    );

    // The pre-reset session must be dead.
    let resp = client.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "all sessions must be invalidated by a password reset"
    );

    // Old password no longer works; the new one does.
    let fresh = TestClient::new();
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "old password");
    // The new password is accepted; the fresh browser is an unknown device, so
    // the login is held for new-device verification (202) rather than issuing a
    // session directly. Credential validity is what this test asserts; the
    // device-verify completion itself is covered elsewhere in this suite.
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": new_password }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED, "new password");
}

/// The native-app login endpoint enforces new-device verification, same as the browser.
///
/// A session must never be minted for valid credentials on an unrecognized device — that would be
/// a full bypass of the browser flow's new-device gate (OWASP ASVS 6.3.5), reachable by anyone who
/// only had the password. The app presents its stored device-recognition token in the
/// `X-Device-Token` header (the app-channel equivalent of the browser device cookie): an
/// unrecognized (or absent) device is held (202) and challenged by email, exactly like the
/// browser. Confirming via `/v0/app/auth/device/verify` returns the session token and a device
/// token in the body; re-presenting that device token logs in directly.
#[tokio::test]
async fn app_login_enforces_new_device_verification() {
    let owner = TestClient::new();
    let user = owner.signup_and_login().await;
    let http = reqwest::Client::new();

    // First app login from an unknown device (no X-Device-Token) must be HELD for new-device
    // verification — no session is handed out.
    let before = e2e::latest_message_id(&user.email).await;
    let resp = http
        .post(format!("{}/v0/app/auth/login", e2e::base_url()))
        .header("X-Turnstile-Token", "e2e-test-token")
        .json(&json!({ "email": user.email, "password": user.password }))
        .send()
        .await
        .expect("app login");
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "app login from an unknown device must be held for new-device verification (202), \
         not issued a session directly"
    );

    // Complete the emailed challenge via the app device-verify endpoint: it returns BOTH the
    // session token and the device token in the body (an app has no cookie jar).
    let body = e2e::wait_for_new_message_body(&user.email, before.as_deref()).await;
    let verify_token = e2e::extract_token(&body).expect("device verification token");
    let resp = http
        .post(format!("{}/v0/app/auth/device/verify", e2e::base_url()))
        .header("X-Turnstile-Token", "e2e-test-token")
        .json(&json!({ "token": verify_token }))
        .send()
        .await
        .expect("app device verify");
    assert_eq!(resp.status(), StatusCode::OK, "app device verify");
    let body: Value = resp.json().await.expect("device verify body is JSON");
    let session_token = body["token"]
        .as_str()
        .expect("session token in body")
        .to_string();
    let device_token = body["device_token"]
        .as_str()
        .expect("device token in body")
        .to_string();

    // The session minted by the verified flow authenticates.
    let resp = http
        .get(format!("{}/v0/user/me", e2e::base_url()))
        .header("Authorization", format!("Bearer {session_token}"))
        .send()
        .await
        .expect("bearer GET /v0/user/me");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "verified app session authenticates"
    );

    // A second app login that presents the stored device token is recognized and logs in
    // directly (200) — no fresh challenge.
    let resp = http
        .post(format!("{}/v0/app/auth/login", e2e::base_url()))
        .header("X-Turnstile-Token", "e2e-test-token")
        .header("X-Device-Token", &device_token)
        .json(&json!({ "email": user.email, "password": user.password }))
        .send()
        .await
        .expect("app login (known device)");
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "a recognized app device (X-Device-Token) must log in directly"
    );
    let body: Value = resp.json().await.expect("login body is JSON");
    assert!(
        body["token"].as_str().is_some(),
        "recognized app login returns a session token"
    );
}

#[tokio::test]
async fn session_listing_and_revocation_are_per_user() {
    // Alice logs in from two "browsers".
    let alice1 = TestClient::new();
    let alice = alice1.signup_and_login().await;
    let alice2 = TestClient::new();
    // The second browser is an unknown device: the login is held (202) and the
    // session only exists after the emailed device challenge is completed.
    let before_device_mail = e2e::latest_message_id(&alice.email).await;
    let resp = alice2
        .post_json(
            "/v0/auth/login",
            &json!({ "email": alice.email, "password": alice.password }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
    let body = e2e::wait_for_new_message_body(&alice.email, before_device_mail.as_deref()).await;
    let device_token = e2e::extract_token(&body).expect("device verification token");
    let resp = alice2
        .post_json("/v0/auth/device/verify", &json!({ "token": device_token }))
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "device verify");

    let resp = alice1.get("/v0/auth/sessions").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let sessions = body["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 2, "both sessions must be listed");
    let current: Vec<&Value> = sessions
        .iter()
        .filter(|s| s["is_current"].as_bool() == Some(true))
        .collect();
    assert_eq!(current.len(), 1, "exactly one session is the current one");

    let other = sessions
        .iter()
        .find(|s| s["is_current"].as_bool() == Some(false))
        .expect("the second session");
    let other_id = other["management_id"].as_str().unwrap();
    let current_id = current[0]["management_id"].as_str().unwrap();

    // A different user revoking Alice's session gets 404 — not 403 — so session ids
    // can't even be probed for existence.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let resp = bob.delete(&format!("/v0/auth/sessions/{current_id}")).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "foreign session revocation must be 404"
    );

    // Alice revokes her other session; that browser is logged out.
    let resp = alice1
        .delete(&format!("/v0/auth/sessions/{other_id}"))
        .await;
    assert!(
        resp.status().is_success(),
        "own-session revoke failed: {}",
        resp.status()
    );
    let resp = alice2.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "the revoked session must be dead"
    );
    // The revoking session itself stays alive.
    let resp = alice1.get("/v0/user/me").await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn change_password_requires_the_current_password() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    let resp = client
        .post_json(
            "/v0/auth/change-password",
            &json!({ "current_password": "wrong-password", "new_password": "e2e-changed-789" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "a wrong current password must be rejected"
    );

    let resp = client
        .post_json(
            "/v0/auth/change-password",
            &json!({ "current_password": user.password, "new_password": "e2e-changed-789" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "change-password failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    // The new password authenticates (202: credentials accepted, held for
    // new-device verification — this fresh browser is unknown).
    let fresh = TestClient::new();
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": "e2e-changed-789" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn logout_invalidates_the_current_session() {
    let client = TestClient::new();
    client.signup_and_login().await;

    let resp = client.get("/v0/user/me").await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client.post_json("/v0/auth/logout", &json!({})).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "logout should return 204"
    );

    let resp = client.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "the logged-out session must not authenticate future requests"
    );
}

#[tokio::test]
async fn resend_verification_email_is_pending_only_and_enumeration_safe() {
    let client = TestClient::new();
    let suffix = e2e::unique();
    let handle = format!("p{}", &suffix[..12]);
    let email = format!("{handle}@test.invalid");
    let password = "e2e-password-123";

    let before_signup = e2e::latest_message_id(&email).await;
    let resp = client
        .post_json(
            "/v0/auth/signup",
            &json!({
                "email": email,
                "handle": handle,
                "display_name": "Pending E2E User",
                "password": password,
            }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "signup should create a pending verification"
    );

    let first_body = e2e::wait_for_new_message_body(&email, before_signup.as_deref()).await;
    let first_token = e2e::extract_token(&first_body).expect("first verification token");
    let first_message_id = e2e::latest_message_id(&email).await;

    let resp = client
        .post_json(
            "/v0/auth/resend-verification-email",
            &json!({ "email": email }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "resend for a pending signup should be 204"
    );

    let second_body = e2e::wait_for_new_message_body(&email, first_message_id.as_deref()).await;
    let second_token = e2e::extract_token(&second_body).expect("resent verification token");
    assert_ne!(
        second_token, first_token,
        "resend should issue a fresh token (only the token hash is stored, never the raw token)"
    );

    // The previous link is invalidated by the reissue...
    let resp = client
        .post_json("/v0/auth/verify-email", &json!({ "token": first_token }))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "the old verification link must stop working after a resend"
    );

    // ...and the freshly issued link completes the signup.
    let resp = client
        .post_json("/v0/auth/verify-email", &json!({ "token": second_token }))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "the resent verification link must complete signup"
    );

    let unknown_email = format!("missing-{}@test.invalid", &e2e::unique()[..12]);
    let resp = client
        .post_json(
            "/v0/auth/resend-verification-email",
            &json!({ "email": unknown_email }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "unknown emails must get the same 204 response to avoid enumeration"
    );
}
