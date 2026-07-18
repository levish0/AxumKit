//! Public user-surface e2e tests: handle availability, public profiles by handle
//! and by id, and the masking policy for banned/deactivated accounts. Run via
//! `just e2e`.
//!
//! Policy references:
//! - `/v0/users/handle/{handle}/available`, `/v0/users/profile?handle=` and
//!   `/v0/users/profile/id?user_id=` are public.
//! - A public profile must NEVER include the account email (PII stays on
//!   `/v0/user/me`, which is session-scoped).
//! - Deactivated (self-deleted) accounts keep handle/display_name exposed for
//!   attribution while the private profile fields (bio/images) are masked, and
//!   the handle stays permanently reserved.
//! - An active ban is reported on the public profile (`is_banned`, `banned_until`,
//!   `ban_reason`) without deactivating the account.

use e2e::TestClient;
use entity::common::Role;
use reqwest::StatusCode;
use serde_json::json;

#[tokio::test]
async fn handle_availability_reflects_signups() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    let anon = TestClient::new();
    let resp = anon
        .get(&format!("/v0/users/handle/{}/available", user.handle))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["available"].as_bool(), Some(false), "taken handle");

    let free = format!("h{}", &e2e::unique()[..12]);
    let resp = anon
        .get(&format!("/v0/users/handle/{free}/available"))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["available"].as_bool(), Some(true), "free handle");
}

#[tokio::test]
async fn public_profile_never_exposes_the_email() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    let anon = TestClient::new();
    let resp = anon
        .get_q("/v0/users/profile", &[("handle", user.handle.as_str())])
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["handle"].as_str(), Some(user.handle.as_str()));
    assert!(
        body.get("email").is_none() || body["email"].is_null(),
        "public profiles must not leak the email: {body}"
    );
    let serialized = body.to_string();
    assert!(
        !serialized.contains(&user.email),
        "public profile payload must not contain the email anywhere"
    );
}

#[tokio::test]
async fn public_profile_by_id_matches_handle_lookup() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Resolve the user's id through the public handle lookup first.
    let anon = TestClient::new();
    let resp = anon
        .get_q("/v0/users/profile", &[("handle", user.handle.as_str())])
        .await;
    let by_handle = TestClient::json_ok(resp, StatusCode::OK).await;
    let user_id = by_handle["id"].as_str().expect("profile exposes id");

    let resp = anon
        .get_q("/v0/users/profile/id", &[("user_id", user_id)])
        .await;
    let by_id = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        by_id["handle"].as_str(),
        Some(user.handle.as_str()),
        "id lookup must resolve to the same user: {by_id}"
    );
    // The by-id surface is bound to the same masking policy: no email, ever.
    let serialized = by_id.to_string();
    assert!(
        !serialized.contains(&user.email),
        "public profile payload must not contain the email anywhere"
    );

    // An unknown id is a clean 404, not an error leak.
    let resp = anon
        .get_q(
            "/v0/users/profile/id",
            &[("user_id", "00000000-0000-4000-8000-000000000000")],
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn banned_user_profile_reports_ban_status() {
    let target = TestClient::new();
    let target_user = target.signup_and_login().await;

    let admin = TestClient::new();
    let admin_user = admin.signup_and_login().await;
    e2e::grant_role(&admin_user.handle, Role::Admin).await;

    // Resolve the target's id via the public profile.
    let anon = TestClient::new();
    let resp = anon
        .get_q(
            "/v0/users/profile",
            &[("handle", target_user.handle.as_str())],
        )
        .await;
    let profile = TestClient::json_ok(resp, StatusCode::OK).await;
    let user_id = profile["id"].as_str().expect("profile exposes id");
    assert_eq!(
        profile["is_banned"].as_bool(),
        Some(false),
        "fresh account must not be banned: {profile}"
    );

    // Permanent ban (no expires_at).
    let resp = admin
        .post_json(
            "/v0/users/ban",
            &json!({ "user_id": user_id, "reason": "e2e ban reason" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "admin ban should succeed; body: {}",
        resp.text().await.unwrap_or_default()
    );

    let resp = anon
        .get_q(
            "/v0/users/profile",
            &[("handle", target_user.handle.as_str())],
        )
        .await;
    let profile = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        profile["is_banned"].as_bool(),
        Some(true),
        "ban must be visible on the public profile: {profile}"
    );
    assert_eq!(
        profile["ban_reason"].as_str(),
        Some("e2e ban reason"),
        "ban reason must be echoed: {profile}"
    );
    assert!(
        profile.get("banned_until").is_none() || profile["banned_until"].is_null(),
        "a permanent ban has no expiry: {profile}"
    );
    // A ban does not deactivate the account; identity stays intact.
    assert_eq!(profile["deactivated"].as_bool(), Some(false));
    assert_eq!(
        profile["handle"].as_str(),
        Some(target_user.handle.as_str())
    );
}

#[tokio::test]
async fn deactivated_profile_masks_private_fields_but_keeps_identity() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Self-service deletion: password accounts re-authenticate inline and get an
    // immediate 204 (soft delete — the row is scrubbed and marked deactivated).
    let resp = client
        .delete_json("/v0/user/me", &json!({ "password": user.password }))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "account deletion should succeed"
    );

    let anon = TestClient::new();
    let resp = anon
        .get_q("/v0/users/profile", &[("handle", user.handle.as_str())])
        .await;
    let profile = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        profile["deactivated"].as_bool(),
        Some(true),
        "deleted account must read as deactivated: {profile}"
    );
    // Identity is kept for attribution...
    assert_eq!(profile["handle"].as_str(), Some(user.handle.as_str()));
    assert!(
        profile["display_name"].is_string(),
        "display_name stays exposed: {profile}"
    );
    // ...while private profile fields are masked.
    assert!(
        profile.get("bio").is_none() || profile["bio"].is_null(),
        "bio must be masked on a deactivated profile: {profile}"
    );
    assert!(
        profile.get("profile_image").is_none() || profile["profile_image"].is_null(),
        "profile image must be masked on a deactivated profile: {profile}"
    );
    // The (scrubbed) email must not surface anywhere either.
    let serialized = profile.to_string();
    assert!(
        !serialized.contains(&user.email),
        "deactivated profile must not contain the original email"
    );

    // The handle stays permanently reserved — deletion does not free it.
    let resp = anon
        .get(&format!("/v0/users/handle/{}/available", user.handle))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["available"].as_bool(),
        Some(false),
        "a deactivated account's handle stays reserved: {body}"
    );
}
