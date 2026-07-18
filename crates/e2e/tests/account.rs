//! Account lifecycle e2e tests: email change and account deletion.
//! Run via `just e2e`.
//!
//! Policy references:
//! - Email change requires the current password; a confirmation token goes to the NEW
//!   address and only `confirm-email-change` applies it. Confirming invalidates every
//!   active session, and the old email no longer authenticates.
//! - A login from an unknown browser is held for new-device email verification
//!   (ASVS 6.3.5): 202 + token mail, completed at POST /v0/auth/device/verify.
//! - Account deletion (DELETE /v0/user/me) requires re-authentication (ASVS 7.5.1 —
//!   password accounts supply `password`), scrubs PII and kills the session/login.
//!   The user row survives as a deactivated shell: the public profile is masked and
//!   the handle stays permanently reserved.

use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn email_change_requires_password_and_confirmation() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;
    let new_email = format!("changed-{}@test.invalid", &e2e::unique()[..12]);

    // A wrong password cannot start an email change.
    let resp = client
        .post_json(
            "/v0/auth/change-email",
            &json!({ "password": "wrong-password", "new_email": new_email }),
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "email change with a wrong password must fail, got {}",
        resp.status()
    );

    let resp = client
        .post_json(
            "/v0/auth/change-email",
            &json!({ "password": user.password, "new_email": new_email }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "change-email failed: {}",
        resp.status()
    );

    // The confirmation token is sent to the NEW address.
    let body = e2e::wait_for_latest_message_body(&new_email).await;
    let token = e2e::extract_token(&body).expect("email change token");
    let before_confirm = e2e::latest_message_id(&new_email).await;
    let resp = client
        .post_json("/v0/auth/confirm-email-change", &json!({ "token": token }))
        .await;
    assert!(
        resp.status().is_success(),
        "confirm-email-change failed: {}",
        resp.status()
    );

    // Changing the login identifier invalidates every active session, including the
    // one that requested the change: a hijacker must not silently retain access.
    let resp = client.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "sessions must be invalidated after an email change"
    );

    // The change also alerts the new address (ASVS 6.3.7). Consume that mail now so
    // the device-verification message below cannot be confused with it.
    let _alert = e2e::wait_for_new_message_body(&new_email, before_confirm.as_deref()).await;

    // Only the new email authenticates from now on.
    let fresh = TestClient::new();
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "old email");

    // The new credentials are accepted, but this fresh browser is an unknown
    // device, so the login is held for new-device email verification
    // (ASVS 6.3.5) instead of creating a session directly.
    let before_device_mail = e2e::latest_message_id(&new_email).await;
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": new_email, "password": user.password }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "new email + unknown device must be held for verification"
    );

    // Complete the device challenge with the token mailed to the (new) address.
    let body = e2e::wait_for_new_message_body(&new_email, before_device_mail.as_deref()).await;
    let device_token = e2e::extract_token(&body).expect("device verification token");
    let resp = fresh
        .post_json("/v0/auth/device/verify", &json!({ "token": device_token }))
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "device verify");

    // The verified fresh browser now has a working session.
    let resp = fresh.get("/v0/user/me").await;
    assert!(
        resp.status().is_success(),
        "session after device verify: {}",
        resp.status()
    );
}

#[tokio::test]
async fn account_deletion_kills_login_and_masks_profile() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Deletion requires re-authentication (ASVS 7.5.1): a bare DELETE without the
    // password must be rejected.
    let resp = client.delete("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "deletion without re-authentication must be 401"
    );

    // Password accounts re-authenticate inline; deletion completes immediately.
    let resp = client
        .delete_json("/v0/user/me", &json!({ "password": user.password }))
        .await;
    assert!(
        resp.status().is_success(),
        "account deletion failed: {}",
        resp.status()
    );

    // The session and the credentials are dead.
    let resp = client.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "session after delete"
    );
    let fresh = TestClient::new();
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "deleted accounts must not authenticate"
    );

    // The public profile survives as a deactivated shell: handle/display_name stay
    // for attribution, but bio and media are masked.
    let anon = TestClient::new();
    let resp = anon
        .get_q("/v0/users/profile", &[("handle", &user.handle)])
        .await;
    let profile = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        profile["deactivated"], true,
        "deleted account must be marked deactivated: {profile}"
    );
    assert_eq!(profile["handle"], user.handle.as_str(), "handle kept");
    assert!(profile["bio"].is_null(), "bio must be masked: {profile}");
    assert!(
        profile["profile_image"].is_null(),
        "profile image must be masked: {profile}"
    );

    // The scrubbed row keeps the handle, so it stays permanently reserved: a new
    // signup cannot claim the deleted user's identity.
    let resp = fresh
        .post_json(
            "/v0/auth/signup",
            &json!({
                "email": format!("reclaim-{}@test.invalid", &e2e::unique()[..12]),
                "handle": user.handle,
                "display_name": "Handle Squatter",
                "password": "e2e-password-123",
            }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "deleted handles must stay reserved"
    );
}

#[tokio::test]
async fn deletion_confirm_rejects_garbage_token() {
    // The email-token confirm endpoint is public (the emailed single-use token is the
    // re-authentication proof), so a made-up token must never delete anything.
    let anon = TestClient::new();
    let resp = anon
        .post_json(
            "/v0/user/me/deletion/confirm",
            &json!({ "token": format!("bogus-{}", e2e::unique()) }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "garbage deletion token must be rejected"
    );
}
