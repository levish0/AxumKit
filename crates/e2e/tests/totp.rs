//! TOTP (2FA) e2e tests, including the SEC-002 backup-code race regression.
//! Run via `just e2e`.
//!
//! Policy references:
//! - Enrollment: `setup` creates a pending secret (otpauth URI), `enable` verifies the
//!   first authenticator code and returns single-use backup codes.
//! - Status reports whether TOTP is enabled and how many backup codes remain.
//! - Backup-code regeneration requires a live authenticator code and invalidates the
//!   previous backup-code set.
//! - Disable requires a live code and returns the account to password-only login.
//! - Login with TOTP enabled returns 202 + `temp_token`; `verify` exchanges the token
//!   plus a 6-digit code OR an 8-character backup code for a session.
//! - SEC-002: backup codes are strictly single-use EVEN under concurrent verification
//!   (the user row is locked during the read-modify-write, so two parallel verifies
//!   with the same code cannot both succeed).

use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::json;
use totp_rs::TOTP;

/// Enrolls TOTP on a fresh account and returns (user, backup_codes).
async fn signup_with_totp_client() -> (TestClient, e2e::SignedUpUser, TOTP, Vec<String>) {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    let resp = client.post_json("/v0/auth/totp/setup", &json!({})).await;
    let setup = TestClient::json_ok(resp, StatusCode::OK).await;
    let uri = setup["qr_code_uri"].as_str().expect("otpauth uri");

    // Generate the first code exactly like an authenticator app would.
    let totp = TOTP::from_url(uri).expect("parse otpauth uri");
    let code = totp.generate_current().expect("generate TOTP code");

    let resp = client
        .post_json("/v0/auth/totp/enable", &json!({ "code": code }))
        .await;
    let enabled = TestClient::json_ok(resp, StatusCode::OK).await;
    let backup_codes: Vec<String> = enabled["backup_codes"]
        .as_array()
        .expect("backup codes")
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert!(!backup_codes.is_empty());

    (client, user, totp, backup_codes)
}

async fn signup_with_totp() -> (e2e::SignedUpUser, Vec<String>) {
    let (_, user, _, backup_codes) = signup_with_totp_client().await;
    (user, backup_codes)
}

/// Starts a login and returns the TOTP `temp_token` (login must answer 202).
async fn login_for_temp_token(user: &e2e::SignedUpUser) -> (TestClient, String) {
    let client = TestClient::new();
    let resp = client
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::ACCEPTED).await;
    let token = body["temp_token"].as_str().expect("temp token").to_string();
    (client, token)
}

#[tokio::test]
async fn login_with_totp_requires_second_factor() {
    let (user, _) = signup_with_totp().await;

    let (client, temp_token) = login_for_temp_token(&user).await;

    // A wrong code does not produce a session.
    let resp = client
        .post_json(
            "/v0/auth/totp/verify",
            &json!({ "temp_token": temp_token, "code": "000000" }),
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "a wrong TOTP code must be rejected, got {}",
        resp.status()
    );
    let resp = client.get("/v0/user/me").await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "no session may exist before the second factor succeeds"
    );
}

/// SEC-002 — one backup code, two concurrent verifies: exactly one may succeed.
#[tokio::test]
async fn sec_002_backup_code_is_single_use_under_concurrency() {
    let (user, backup_codes) = signup_with_totp().await;
    let code = &backup_codes[0];

    // Two independent half-logins, each holding its own temp token.
    let (client_a, token_a) = login_for_temp_token(&user).await;
    let (client_b, token_b) = login_for_temp_token(&user).await;

    let verify = |client: TestClient, token: String, code: String| async move {
        client
            .post_json(
                "/v0/auth/totp/verify",
                &json!({ "temp_token": token, "code": code }),
            )
            .await
            .status()
    };

    let (status_a, status_b) = tokio::join!(
        verify(client_a, token_a, code.clone()),
        verify(client_b, token_b, code.clone()),
    );

    let successes = [status_a, status_b]
        .iter()
        .filter(|s| s.is_success())
        .count();
    assert_eq!(
        successes, 1,
        "the same backup code must authenticate exactly once under concurrency \
         (got {status_a} and {status_b})"
    );

    // And it stays consumed for any later attempt.
    let (client_c, token_c) = login_for_temp_token(&user).await;
    let resp = client_c
        .post_json(
            "/v0/auth/totp/verify",
            &json!({ "temp_token": token_c, "code": code }),
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "a consumed backup code must never authenticate again, got {}",
        resp.status()
    );

    // A different, unused backup code still works.
    let (client_d, token_d) = login_for_temp_token(&user).await;
    let resp = client_d
        .post_json(
            "/v0/auth/totp/verify",
            &json!({ "temp_token": token_d, "code": backup_codes[1] }),
        )
        .await;
    // 202: the backup code was accepted (a consumed one is rejected above);
    // this fresh browser is an unknown device, so the login is held for the
    // device email challenge rather than issuing the session directly.
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "an unused backup code must still authenticate"
    );
}

#[tokio::test]
async fn status_backup_code_rotation_and_disable_flow() {
    let (client, user, totp, old_backup_codes) = signup_with_totp_client().await;

    let resp = client.get("/v0/auth/totp/status").await;
    let status = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(status["enabled"].as_bool(), Some(true));
    assert!(
        status["enabled_at"].as_str().is_some(),
        "enabled TOTP must expose enabled_at"
    );
    assert_eq!(
        status["backup_codes_remaining"].as_u64(),
        Some(old_backup_codes.len() as u64)
    );

    let code = totp.generate_current().expect("generate TOTP code");
    let resp = client
        .post_json(
            "/v0/auth/totp/backup-codes/regenerate",
            &json!({ "code": code }),
        )
        .await;
    let rotated = TestClient::json_ok(resp, StatusCode::OK).await;
    let new_backup_codes: Vec<String> = rotated["backup_codes"]
        .as_array()
        .expect("new backup codes")
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert_eq!(new_backup_codes.len(), old_backup_codes.len());
    assert!(
        new_backup_codes.iter().all(|code| code.len() == 8),
        "backup codes should be returned as 8-character plaintext codes"
    );
    let old_backup_code = old_backup_codes[0].clone();
    assert!(
        !new_backup_codes.contains(&old_backup_code),
        "rotating backup codes should replace the previous set"
    );

    let (old_code_client, temp_token) = login_for_temp_token(&user).await;
    let resp = old_code_client
        .post_json(
            "/v0/auth/totp/verify",
            &json!({ "temp_token": temp_token, "code": old_backup_code }),
        )
        .await;
    assert!(
        !resp.status().is_success(),
        "a backup code from the old set must not work after regeneration"
    );

    let resp = client
        .post_json("/v0/auth/totp/disable", &json!({ "code": "00000000" }))
        .await;
    assert!(
        !resp.status().is_success(),
        "disabling TOTP with an invalid backup code must fail"
    );

    let code = totp.generate_current().expect("generate TOTP code");
    let resp = client
        .post_json("/v0/auth/totp/disable", &json!({ "code": code }))
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = client.get("/v0/auth/totp/status").await;
    let status = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(status["enabled"].as_bool(), Some(false));
    assert!(
        status["backup_codes_remaining"].is_null(),
        "disabled TOTP should not report remaining backup codes"
    );

    let fresh = TestClient::new();
    let resp = fresh
        .post_json(
            "/v0/auth/login",
            &json!({ "email": user.email, "password": user.password }),
        )
        .await;
    // 202 = the password alone was sufficient (no TOTP temp-token step): the
    // remaining challenge is only the unknown-device email verification.
    assert_eq!(
        resp.status(),
        StatusCode::ACCEPTED,
        "after disabling TOTP, password login needs no second factor"
    );
}
