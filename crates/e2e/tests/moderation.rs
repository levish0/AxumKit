//! Moderation e2e tests: user bans and role management. Run via `just e2e`.
//!
//! Policy references:
//! - `POST /v0/users/ban` / `unban` — admin-only (`require_admin_for_target`), which
//!   also forbids self-targeting (403 CannotManageSelf) and targeting fellow admins
//!   (403 CannotManageHigherOrEqualRole).
//! - A banned user fails every `require_not_banned` gate: board posts, edits,
//!   comments, ... (403 UserBanned). Reads stay open — a ban gates participation,
//!   not reading. Unban restores write access.
//! - `POST /v0/users/roles/grant` / `revoke` — same admin-only target gate, so the
//!   Admin role can never be revoked (any admin target is refused) and no role can
//!   be self-managed.
//! - `GET /v0/moderation/logs` is gated at the router boundary by `require_mod`.

use e2e::TestClient;
use entity::common::Role;
use reqwest::StatusCode;
use serde_json::json;

async fn new_admin() -> TestClient {
    let admin = TestClient::new();
    let user = admin.signup_and_login().await;
    e2e::grant_role(&user.handle, Role::Admin).await;
    admin
}

/// Resolves a seeded board's id by slug.
async fn board_id(client: &TestClient, slug: &str) -> String {
    let resp = client.get_q("/v0/board/by-slug", &[("slug", slug)]).await;
    let board = TestClient::json_ok(resp, StatusCode::OK).await;
    board["id"].as_str().expect("board id").to_string()
}

#[tokio::test]
async fn banned_user_loses_board_write_access_until_unbanned() {
    let alice = TestClient::new();
    alice.signup_and_login().await;
    let alice_id = alice.me().await["id"].as_str().unwrap().to_string();

    let general = board_id(&alice, "general").await;
    let resp = alice
        .post_json(
            "/v0/board/post",
            &json!({
                "board_id": general,
                "title": format!("Ban {}", e2e::unique()),
                "content": "v1",
            }),
        )
        .await;
    let post = TestClient::json_ok(resp, StatusCode::CREATED).await;
    let post_id = post["id"].as_str().unwrap().to_string();

    // Banning is admin-only: a regular user cannot ban.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let resp = bob
        .post_json(
            "/v0/users/ban",
            &json!({ "user_id": alice_id, "reason": "bob must not ban" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    let admin = new_admin().await;
    let resp = admin
        .post_json(
            "/v0/users/ban",
            &json!({ "user_id": alice_id, "reason": "e2e: ban test" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "admin ban failed: {}",
        resp.status()
    );

    // While banned, every board write surface rejects Alice with 403.
    let resp = alice
        .post_json(
            "/v0/board/post",
            &json!({
                "board_id": general,
                "title": format!("Banned {}", e2e::unique()),
                "content": "must fail",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "banned post create");
    let resp = alice
        .post_json(
            "/v0/board/post/update",
            &json!({ "post_id": post_id, "content": "v2" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "banned edit");
    let resp = alice
        .post_json(
            "/v0/board/comment",
            &json!({ "post_id": post_id, "content": "banned comment" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "banned comment");

    // Reads stay open while banned: a ban gates participation, not reading.
    let resp = alice
        .get_q(
            "/v0/board/post/list",
            &[
                ("board_id", general.as_str()),
                ("page", "1"),
                ("page_size", "10"),
            ],
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "banned list read");
    let resp = alice
        .get_q("/v0/board/post", &[("post_id", post_id.as_str())])
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "banned post read");

    // Unban restores write access.
    let resp = admin
        .post_json(
            "/v0/users/unban",
            &json!({ "user_id": alice_id, "reason": "e2e: unban" }),
        )
        .await;
    assert!(resp.status().is_success());
    let resp = alice
        .post_json(
            "/v0/board/post/update",
            &json!({ "post_id": post_id, "content": "v2" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "edit must work again after unban"
    );
}

#[tokio::test]
async fn admins_cannot_ban_themselves_or_other_admins() {
    let admin = new_admin().await;
    let admin_id = admin.me().await["id"].as_str().unwrap().to_string();

    let other_admin = new_admin().await;
    let other_admin_id = other_admin.me().await["id"].as_str().unwrap().to_string();

    let resp = admin
        .post_json(
            "/v0/users/ban",
            &json!({ "user_id": admin_id, "reason": "self-ban must fail" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "self-ban must be rejected"
    );

    let resp = admin
        .post_json(
            "/v0/users/ban",
            &json!({ "user_id": other_admin_id, "reason": "admin-vs-admin must fail" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "banning a fellow admin must be rejected"
    );
}

/// Role grant/revoke is admin-only and target-gated: self-management and admin
/// targets are always refused — which is precisely why the Admin role can never be
/// revoked through the API (any holder of it is an unmanageable target). A Mod
/// grant to a plain user takes effect on the target's next request (roles are read
/// per request), and revoking it takes the access away again.
#[tokio::test]
async fn role_management_rejects_self_and_admin_targets() {
    let admin = new_admin().await;
    let admin_id = admin.me().await["id"].as_str().unwrap().to_string();

    let other_admin = new_admin().await;
    let other_admin_id = other_admin.me().await["id"].as_str().unwrap().to_string();

    let carol = TestClient::new();
    carol.signup_and_login().await;
    let carol_id = carol.me().await["id"].as_str().unwrap().to_string();

    // A regular user cannot grant roles at all.
    let resp = carol
        .post_json(
            "/v0/users/roles/grant",
            &json!({ "user_id": admin_id, "role": "mod", "reason": "not allowed" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "non-admin grant");

    // Admin self-management is refused for every role (no self-grant loophole).
    let resp = admin
        .post_json(
            "/v0/users/roles/grant",
            &json!({ "user_id": admin_id, "role": "mod", "reason": "self grant must fail" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "self grant");

    // Fellow admins are unmanageable targets: neither grant nor revoke touches them,
    // so the Admin role itself can never be revoked (last-admin lockout protection).
    let resp = admin
        .post_json(
            "/v0/users/roles/grant",
            &json!({ "user_id": other_admin_id, "role": "mod", "reason": "admin target must fail" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "grant to admin target"
    );
    let resp = admin
        .post_json(
            "/v0/users/roles/revoke",
            &json!({ "user_id": other_admin_id, "role": "admin", "reason": "admin demote must fail" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "revoke admin role");

    // Positive path: granting Mod to a plain user opens the moderator surface...
    let resp = admin
        .post_json(
            "/v0/users/roles/grant",
            &json!({ "user_id": carol_id, "role": "mod", "reason": "e2e: promote to mod" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["role"], "mod");

    let resp = carol.get("/v0/moderation/logs?limit=10").await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "freshly granted Mod must reach the moderation surface"
    );

    // ...and revoking it closes the surface again on the very next request.
    let resp = admin
        .post_json(
            "/v0/users/roles/revoke",
            &json!({ "user_id": carol_id, "role": "mod", "reason": "e2e: demote" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "revoking Mod from a plain user must succeed, got {}",
        resp.status()
    );
    let resp = carol.get("/v0/moderation/logs?limit=10").await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "revoked Mod must lose the moderation surface"
    );
}

#[tokio::test]
async fn moderation_logs_require_moderator_role() {
    // Anonymous callers are unauthenticated, not merely unauthorized.
    let anon = TestClient::new();
    let resp = anon.get("/v0/moderation/logs?limit=10").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "anonymous logs");

    // A plain signed-in user is refused.
    let user = TestClient::new();
    user.signup_and_login().await;
    let resp = user.get("/v0/moderation/logs?limit=10").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "plain user logs");

    // A moderator can list the logs.
    let moderator = TestClient::new();
    let mod_user = moderator.signup_and_login().await;
    e2e::grant_role(&mod_user.handle, Role::Mod).await;
    let resp = moderator.get("/v0/moderation/logs?limit=10").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(body["data"].is_array(), "logs body must contain data array");
}
