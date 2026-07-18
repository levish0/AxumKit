//! RBAC e2e tests: ACL group CRUD, memberships (with expiry), the permission
//! catalog, and group-permission grants observed through a real endpoint
//! effect. Run via `just e2e`.
//!
//! Policy references (from `crates/server/src/permission/*`,
//! `crates/server/src/service/groups/*`):
//! - Group/member/permission reads require `Mod` (or admin); every mutation is
//!   admin-only. Anonymous callers get 401.
//! - Permissions are Django-style codenames (`board:pin_post`, `board:moderate`,
//!   ...). `Permission::ALL` is served by `GET /v0/permissions`; a codename
//!   the application does not define is rejected (`permission:invalid`) so typos
//!   never become silent dead grants.
//! - A plain user holds a permission only through an active group membership.
//!   Missing permissions surface as 403 `permission:denied` with the codename in
//!   `details`.
//! - The board post pin/unpin/lock/unlock endpoints are all gated by the
//!   `board:moderate` codename (`BoardPermission::Moderate`), so that is the
//!   grant these tests probe through `/v0/board/post/pin`.
//! - Memberships support `expires_at`; expiry is evaluated at request time, so
//!   an expired membership stops granting without any cleanup job.
//! - System groups carry code-known meaning: they cannot be deleted and their
//!   membership cannot be edited through the generic ACL admin API
//!   (`group:is_system`). The API itself only ever creates non-system
//!   groups, so the test seeds one out-of-band.

use e2e::TestClient;
use entity::common::Role;
use reqwest::StatusCode;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, Set};
use serde_json::{Value, json};
use uuid::Uuid;

async fn new_admin() -> TestClient {
    let admin = TestClient::new();
    let user = admin.signup_and_login().await;
    e2e::grant_role(&user.handle, Role::Admin).await;
    admin
}

async fn new_mod() -> TestClient {
    let moderator = TestClient::new();
    let user = moderator.signup_and_login().await;
    e2e::grant_role(&user.handle, Role::Mod).await;
    moderator
}

/// Seeds a system group straight into the test database. The HTTP API can only
/// create non-system groups (by design), so exercising the system-group
/// protections requires planting one out-of-band — same bootstrap rationale as
/// `e2e::grant_role`. One short-lived, single-connection pool per call, closed
/// deterministically (see the harness notes on per-runtime connections).
async fn seed_system_group(name: &str) -> String {
    let mut opt = ConnectOptions::new(e2e::database_url());
    opt.max_connections(1).min_connections(0);
    let db = Database::connect(opt)
        .await
        .expect("connect to test database");

    let group = entity::groups::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(name.to_string()),
        description: Set(Some("e2e: seeded system group".to_string())),
        is_system: Set(true),
        created_at: Set(chrono::Utc::now()),
    }
    .insert(&db)
    .await
    .expect("insert system group");

    db.close().await.expect("close test db connection");
    group.id.to_string()
}

/// Looks up a seeded board's id by slug (the compose stack seeds "notice",
/// "general", and "qna").
async fn board_id(client: &TestClient, slug: &str) -> String {
    let resp = client.get_q("/v0/board/by-slug", &[("slug", slug)]).await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    body["id"].as_str().unwrap().to_string()
}

/// Creates a post to use as the pin probe target and returns its id.
async fn create_probe_post(client: &TestClient, board_id: &str) -> String {
    let resp = client
        .post_json(
            "/v0/board/post",
            &json!({
                "board_id": board_id,
                "title": format!("Pin probe {}", e2e::unique()),
                "content": "pin probe body",
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;
    body["id"].as_str().unwrap().to_string()
}

/// Attempts to pin `post_id` as `client`.
async fn pin(client: &TestClient, post_id: &str) -> reqwest::Response {
    client
        .post_json(
            "/v0/board/post/pin",
            &json!({ "post_id": post_id, "reason": "e2e: pin probe" }),
        )
        .await
}

/// Asserts a 403 `permission:denied` error body and returns its `details` string
/// (the missing permission codename).
async fn assert_permission_denied(resp: reqwest::Response) -> String {
    let body = TestClient::json_ok(resp, StatusCode::FORBIDDEN).await;
    assert_eq!(
        body["code"].as_str(),
        Some("permission:denied"),
        "expected an RBAC denial: {body}"
    );
    body["details"].as_str().unwrap_or_default().to_string()
}

/// Creates a group as `admin` and returns (group_id, name).
async fn create_group(admin: &TestClient, prefix: &str) -> (String, String) {
    let name = format!("{prefix}-{}", &e2e::unique()[..12]);
    let resp = admin
        .post_json(
            "/v0/groups",
            &json!({
                "name": name,
                "description": "e2e: rbac test group",
                "reason": "e2e: create group",
            }),
        )
        .await;
    let group = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(group["name"].as_str(), Some(name.as_str()));
    assert_eq!(group["is_system"].as_bool(), Some(false));
    (group["id"].as_str().unwrap().to_string(), name)
}

/// Adds `user_id` to `group_id` and returns the membership row id.
async fn add_member(admin: &TestClient, group_id: &str, user_id: &str) -> String {
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({ "group_id": group_id, "user_id": user_id, "reason": "e2e: member" }),
        )
        .await;
    let member = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(member["user_id"].as_str(), Some(user_id));
    member["id"].as_str().unwrap().to_string()
}

/// Replaces `group_id`'s permission grants with `permissions`.
async fn replace_permissions(admin: &TestClient, group_id: &str, permissions: &[&str]) {
    let resp = admin
        .post_json(
            "/v0/groups/permissions/replace",
            &json!({
                "group_id": group_id,
                "permissions": permissions,
                "reason": "e2e: grant permissions",
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let granted: Vec<&str> = body["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(granted, permissions, "replace must echo the end state");
}

/// Scenario 1: the permission catalog is Mod-readable and lists every board
/// codename an admin UI would offer as checkboxes.
#[tokio::test]
async fn permissions_catalog_is_mod_readable_and_lists_board_codenames() {
    // Anonymous callers are rejected outright.
    let anon = TestClient::new();
    let resp = anon.get("/v0/permissions").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "anon catalog read");

    // A plain user is below the Mod bar.
    let user = TestClient::new();
    user.signup_and_login().await;
    let resp = user.get("/v0/permissions").await;
    let body = TestClient::json_ok(resp, StatusCode::FORBIDDEN).await;
    assert_eq!(
        body["code"].as_str(),
        Some("user:permission_insufficient"),
        "catalog read as plain user: {body}"
    );

    // A moderator sees the full catalog.
    let moderator = new_mod().await;
    let resp = moderator.get("/v0/permissions").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let listed: Vec<&str> = body["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for codename in [
        "board:pin_post",
        "board:lock_post",
        "board:moderate",
        "board:manage",
    ] {
        assert!(
            listed.contains(&codename),
            "catalog must include {codename}: {listed:?}"
        );
    }
}

/// Scenario 2: group listing requires Mod; every group mutation is admin-only
/// (even a moderator may not create); delete works exactly once.
#[tokio::test]
async fn group_management_is_admin_gated() {
    let admin = new_admin().await;
    let (group_id, group_name) = create_group(&admin, "e2e-crud").await;

    // Creating the same name again is a conflict, not a silent upsert.
    let resp = admin
        .post_json(
            "/v0/groups",
            &json!({ "name": group_name, "reason": "e2e: duplicate" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CONFLICT).await;
    assert_eq!(
        body["code"].as_str(),
        Some("group:already_exists"),
        "duplicate group name: {body}"
    );

    // A moderator can list groups and sees the new one...
    let moderator = new_mod().await;
    let resp = moderator.get("/v0/groups").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["groups"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["id"].as_str() == Some(group_id.as_str())),
        "created group must appear in the Mod group list"
    );

    // ...but reads are the ceiling of the Mod tier: mutations stay admin-only.
    let resp = moderator
        .post_json(
            "/v0/groups",
            &json!({ "name": format!("e2e-nope-{}", &e2e::unique()[..12]), "reason": "must fail" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "create group as mod");

    // A plain user can neither list nor create; anonymous gets 401.
    let user = TestClient::new();
    user.signup_and_login().await;
    let resp = user.get("/v0/groups").await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "list groups as user");
    let resp = user
        .post_json(
            "/v0/groups",
            &json!({ "name": format!("e2e-nope-{}", &e2e::unique()[..12]), "reason": "must fail" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "create group as user");
    let anon = TestClient::new();
    let resp = anon.get("/v0/groups").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "list groups anon");

    // Delete works once, then the group is genuinely gone.
    let resp = admin
        .post_json(
            "/v0/groups/delete",
            &json!({ "group_id": group_id, "reason": "e2e: cleanup" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "delete group");
    let resp = admin
        .post_json(
            "/v0/groups/delete",
            &json!({ "group_id": group_id, "reason": "e2e: double delete" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::NOT_FOUND).await;
    assert_eq!(
        body["code"].as_str(),
        Some("group:not_found"),
        "second delete: {body}"
    );
}

/// Scenario 3 (the effect probe): a plain user cannot pin a post; a group
/// grant of `board:moderate` (the codename gating pin/unpin) makes pinning
/// work; deleting the group cascades the grant away and the denial returns.
#[tokio::test]
async fn group_permission_grant_gates_post_pinning() {
    let admin = new_admin().await;

    let alice = TestClient::new();
    alice.signup_and_login().await;
    let alice_id = alice.me().await["id"].as_str().unwrap().to_string();

    let general = board_id(&alice, "general").await;
    let post_id = create_probe_post(&alice, &general).await;

    // Before any grant: denied, and the denial names the missing codename.
    let details = assert_permission_denied(pin(&alice, &post_id).await).await;
    assert_eq!(
        details, "board:moderate",
        "the denial must carry the missing permission codename"
    );

    // Admin wires up the grant: group -> member -> permission list.
    let (group_id, _) = create_group(&admin, "e2e-pin").await;
    add_member(&admin, &group_id, &alice_id).await;
    replace_permissions(&admin, &group_id, &["board:moderate"]).await;

    // The grant is visible on the group's permission read.
    let resp = admin
        .get_q("/v0/groups/permissions", &[("group_id", &group_id)])
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p.as_str() == Some("board:moderate")),
        "group permission read must show the grant: {body}"
    );

    // ...and effective on the very next request (no re-login required).
    let resp = pin(&alice, &post_id).await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["is_pinned"].as_bool(), Some(true), "pin under grant");

    let resp = alice
        .post_json(
            "/v0/board/post/unpin",
            &json!({ "post_id": post_id, "reason": "e2e: unpin probe" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["is_pinned"].as_bool(),
        Some(false),
        "unpin under grant"
    );

    // Deleting the group cascades memberships and grants away.
    let resp = admin
        .post_json(
            "/v0/groups/delete",
            &json!({ "group_id": group_id, "reason": "e2e: revoke via delete" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "delete granting group");

    let details = assert_permission_denied(pin(&alice, &post_id).await).await;
    assert_eq!(
        details, "board:moderate",
        "after group deletion the denial must be back"
    );
}

/// Scenario 4: membership mechanics — a member must be exactly one subject,
/// active duplicates conflict, the member list shows the row, and removing the
/// membership revokes the grant immediately.
#[tokio::test]
async fn membership_removal_revokes_the_grant() {
    let admin = new_admin().await;

    let bob = TestClient::new();
    bob.signup_and_login().await;
    let bob_id = bob.me().await["id"].as_str().unwrap().to_string();

    let general = board_id(&bob, "general").await;
    let post_id = create_probe_post(&bob, &general).await;

    let (group_id, _) = create_group(&admin, "e2e-member").await;
    replace_permissions(&admin, &group_id, &["board:moderate"]).await;

    // A member is a user XOR an IP — both (or neither) is an invalid subject.
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({
                "group_id": group_id,
                "user_id": bob_id,
                "ip_address": "203.0.113.7",
                "reason": "must fail",
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::BAD_REQUEST).await;
    assert_eq!(
        body["code"].as_str(),
        Some("permission:invalid"),
        "both subjects: {body}"
    );
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({ "group_id": group_id, "reason": "must fail" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::BAD_REQUEST).await;
    assert_eq!(
        body["code"].as_str(),
        Some("permission:invalid"),
        "no subject: {body}"
    );

    let member_id = add_member(&admin, &group_id, &bob_id).await;

    // An active duplicate is a conflict, not a second row.
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({ "group_id": group_id, "user_id": bob_id, "reason": "duplicate" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CONFLICT).await;
    assert_eq!(
        body["code"].as_str(),
        Some("group:member_already_exists"),
        "duplicate member: {body}"
    );

    // The membership is visible on the (Mod-gated) member list...
    let resp = admin
        .get_q(
            "/v0/groups/members",
            &[("group_id", &group_id), ("limit", "100")],
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["user_id"].as_str() == Some(bob_id.as_str())),
        "member list must show the membership: {body}"
    );

    // ...and effective: bob can pin.
    let resp = pin(&bob, &post_id).await;
    assert_eq!(resp.status(), StatusCode::OK, "pin while a member");

    // Removing the membership revokes the grant on the next request.
    let resp = admin
        .post_json(
            "/v0/groups/members/remove",
            &json!({ "member_id": member_id, "reason": "e2e: remove member" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "remove member");

    let resp = admin
        .get_q(
            "/v0/groups/members",
            &[("group_id", &group_id), ("limit", "100")],
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["user_id"].as_str() != Some(bob_id.as_str())),
        "removed membership must leave the list: {body}"
    );

    assert_permission_denied(pin(&bob, &post_id).await).await;
}

/// Scenario 5: `expires_at` bounds a membership. A past timestamp is rejected
/// at the DTO layer; a short-future one grants until the clock passes it, at
/// which point the permission dies with no cleanup job involved.
#[tokio::test]
async fn membership_expiry_ends_the_grant() {
    let admin = new_admin().await;

    let carol = TestClient::new();
    carol.signup_and_login().await;
    let carol_id = carol.me().await["id"].as_str().unwrap().to_string();

    let general = board_id(&carol, "general").await;
    let post_id = create_probe_post(&carol, &general).await;

    let (group_id, _) = create_group(&admin, "e2e-expiry").await;
    replace_permissions(&admin, &group_id, &["board:moderate"]).await;

    // A membership that would already be expired is refused up front.
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({
                "group_id": group_id,
                "user_id": carol_id,
                "reason": "must fail",
                "expires_at": chrono::Utc::now() - chrono::Duration::minutes(5),
            }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "a past expires_at must be rejected"
    );

    // Grant for a few seconds only.
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(8);
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({
                "group_id": group_id,
                "user_id": carol_id,
                "reason": "e2e: short-lived member",
                "expires_at": expires_at,
            }),
        )
        .await;
    let member = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        member["expires_at"].as_str().is_some(),
        "membership must echo its expiry: {member}"
    );

    // Inside the window the grant works.
    let resp = pin(&carol, &post_id).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "pin inside the expiry window"
    );

    // Poll until the membership lapses: expiry is evaluated per request, so the
    // pin flips to a denial without any admin action. ~30s budget.
    let mut denied = None;
    for _ in 0..120 {
        let resp = pin(&carol, &post_id).await;
        if resp.status() == StatusCode::FORBIDDEN {
            denied = Some(resp);
            break;
        }
        assert_eq!(resp.status(), StatusCode::OK, "pin before expiry");
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    let denied = denied.expect("membership expiry never took effect");
    let details = assert_permission_denied(denied).await;
    assert_eq!(details, "board:moderate", "expired grant denial codename");
}

/// Scenario 6: a codename the application does not define is rejected before
/// anything is written, and permission writes are admin-only.
#[tokio::test]
async fn unknown_permission_codename_is_rejected() {
    let admin = new_admin().await;
    let (group_id, _) = create_group(&admin, "e2e-typo").await;

    let resp = admin
        .post_json(
            "/v0/groups/permissions/replace",
            &json!({
                "group_id": group_id,
                "permissions": ["board:pin_post_typo"],
                "reason": "must fail",
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::BAD_REQUEST).await;
    assert_eq!(
        body["code"].as_str(),
        Some("permission:invalid"),
        "unknown codename must be rejected: {body}"
    );

    // A bad list is all-or-nothing: one typo poisons the whole replace, even
    // when valid codenames ride along.
    let resp = admin
        .post_json(
            "/v0/groups/permissions/replace",
            &json!({
                "group_id": group_id,
                "permissions": ["board:pin_post", "not-a-permission"],
                "reason": "must fail",
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "mixed list");

    // Nothing stuck: the group still has zero grants.
    let resp = admin
        .get_q("/v0/groups/permissions", &[("group_id", &group_id)])
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["permissions"].as_array().map(Vec::len),
        Some(0),
        "rejected replaces must not persist grants: {body}"
    );

    // Reads stop at Mod; the replace itself is admin-only.
    let moderator = new_mod().await;
    let resp = moderator
        .get_q("/v0/groups/permissions", &[("group_id", &group_id)])
        .await;
    assert_eq!(resp.status(), StatusCode::OK, "permission read as mod");
    let resp = moderator
        .post_json(
            "/v0/groups/permissions/replace",
            &json!({ "group_id": group_id, "permissions": ["board:pin_post"], "reason": "must fail" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "permission replace as mod"
    );
}

/// Scenario 7: system groups (seeded out-of-band — the API only creates
/// non-system groups) are visible but frozen: no deletion and no membership
/// edits through the generic admin API, even for an admin.
#[tokio::test]
async fn system_groups_are_immutable_via_the_generic_api() {
    let admin = new_admin().await;
    let admin_id = admin.me().await["id"].as_str().unwrap().to_string();

    let name = format!("e2e-system-{}", &e2e::unique()[..12]);
    let group_id = seed_system_group(&name).await;

    // Visible, and flagged as system, in the group list.
    let resp = admin.get("/v0/groups").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let listed = body["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"].as_str() == Some(group_id.as_str()))
        .unwrap_or_else(|| panic!("seeded system group missing from list: {body}"));
    assert_eq!(listed["is_system"].as_bool(), Some(true));

    // Deletion is refused.
    let resp = admin
        .post_json(
            "/v0/groups/delete",
            &json!({ "group_id": group_id, "reason": "must fail" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::FORBIDDEN).await;
    assert_eq!(
        body["code"].as_str(),
        Some("group:is_system"),
        "system group deletion must fail: {body}"
    );

    // So is membership editing through the generic member API.
    let resp = admin
        .post_json(
            "/v0/groups/members",
            &json!({ "group_id": group_id, "user_id": admin_id, "reason": "must fail" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::FORBIDDEN).await;
    assert_eq!(
        body["code"].as_str(),
        Some("group:is_system"),
        "adding a member to a system group must fail: {body}"
    );
}
