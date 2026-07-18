//! Notification e2e tests. Run via `just e2e`.
//!
//! Policy references:
//! - All `/v0/notifications/*` routes require a session (401 anon).
//! - Commenting on a board post delivers a `board_comment_created` notification to the
//!   post author — but never to the commenter themselves, and mentioned users get a
//!   `user_mentioned` notification instead of a duplicate comment alert.
//! - `@handle` mentions in post/comment content deliver `user_mentioned` notifications
//!   to the mentioned users, skipping the author.
//! - Inbox rows are strictly per-user: mark-as-read/delete match `WHERE id AND user_id`,
//!   so foreign ids yield 404 (existence is not revealed) and never mutate.
//! - Unread count, mark-all-as-read, and delete operate only on the caller's inbox.
//! - Notification channel/action preferences require login and are scoped per user;
//!   disabling an action suppresses new notifications of that action for the recipient.

use std::time::Duration;

use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::{Value, json};

/// Resolves a seeded board's id by slug (seeded slugs: "notice", "general", "qna").
async fn board_id_by_slug(client: &TestClient, slug: &str) -> String {
    let resp = client.get_q("/v0/board/by-slug", &[("slug", slug)]).await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    body["id"].as_str().expect("board id").to_string()
}

/// Creates a post and returns its id.
async fn create_post(client: &TestClient, board_id: &str, title: &str, content: &str) -> String {
    let resp = client
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": title, "content": content }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;
    body["id"].as_str().expect("post id").to_string()
}

/// Creates a top-level comment on a post.
async fn create_comment(client: &TestClient, post_id: &str, content: &str) {
    let resp = client
        .post_json(
            "/v0/board/comment",
            &json!({ "post_id": post_id, "content": content }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "comment creation should succeed"
    );
}

/// Polls the notification inbox until a notification with `action` about `post_id`
/// appears. Notifications are written best-effort during the triggering request, so
/// this usually resolves on the first poll; the budget just absorbs slow CI.
async fn wait_for_post_notification(client: &TestClient, action: &str, post_id: &str) -> Value {
    for _ in 0..120 {
        let resp = client.get("/v0/notifications/list?limit=50").await;
        if resp.status() == StatusCode::OK
            && let Ok(body) = resp.json::<Value>().await
            && let Some(found) = body["data"]
                .as_array()
                .and_then(|list| {
                    list.iter().find(|n| {
                        n["action"].as_str() == Some(action)
                            && n["post_id"].as_str() == Some(post_id)
                    })
                })
                .cloned()
        {
            return found;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    panic!("timed out waiting for a `{action}` notification about post {post_id}");
}

/// Returns every notification with `action` about `post_id` currently in the inbox.
async fn post_notifications(client: &TestClient, action: &str, post_id: &str) -> Vec<Value> {
    let resp = client.get("/v0/notifications/list?limit=50").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    body["data"]
        .as_array()
        .expect("notification list")
        .iter()
        .filter(|n| n["action"].as_str() == Some(action) && n["post_id"].as_str() == Some(post_id))
        .cloned()
        .collect()
}

#[tokio::test]
async fn inbox_requires_login() {
    let anon = TestClient::new();

    let resp = anon.get("/v0/notifications/list?limit=10").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = anon.get("/v0/notifications/unread/count").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let resp = anon
        .post_json(
            "/v0/notifications/mark-as-read",
            &json!({ "notification_id": "0190a000-0000-7000-8000-000000000000" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn comment_notifies_post_author_and_inbox_lifecycle_works() {
    // Bob owns a post; Alice comments on it.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let board_id = board_id_by_slug(&bob, "general").await;
    let title = format!("Inbox {}", e2e::unique());
    let post_id = create_post(&bob, &board_id, &title, "notification target post\n").await;

    let alice = TestClient::new();
    alice.signup_and_login().await;
    create_comment(&alice, &post_id, "a comment for the author's inbox\n").await;

    // The post author must receive an unread board_comment_created notification.
    let notification = wait_for_post_notification(&bob, "board_comment_created", &post_id).await;
    assert_eq!(notification["is_read"].as_bool(), Some(false));
    assert!(
        notification["comment_id"].as_str().is_some(),
        "comment notification should deep-link the comment"
    );
    let notification_id = notification["id"].as_str().unwrap();

    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["count"].as_u64(),
        Some(1),
        "Bob should have exactly one unread notification"
    );

    // The commenter must NOT be notified about their own comment.
    let resp = alice.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["count"].as_u64(),
        Some(0),
        "the commenter must not receive a notification for their own comment"
    );

    // Bob marks it read; a second attempt is 404 (already read).
    let resp = bob
        .post_json(
            "/v0/notifications/mark-as-read",
            &json!({ "notification_id": notification_id }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["count"].as_u64(), Some(0));

    let resp = bob
        .post_json("/v0/notifications/mark-all-as-read", &json!({}))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "mark-all-as-read should be idempotent for an already-read inbox"
    );

    let resp = bob
        .post_json(
            "/v0/notifications/mark-as-read",
            &json!({ "notification_id": notification_id }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // Delete removes the row; a repeat delete is 404.
    let resp = bob
        .post_json(
            "/v0/notifications/delete",
            &json!({ "notification_id": notification_id }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = bob
        .post_json(
            "/v0/notifications/delete",
            &json!({ "notification_id": notification_id }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "a deleted notification should no longer be addressable"
    );
}

#[tokio::test]
async fn mentions_in_posts_and_comments_notify_mentioned_users() {
    let alice = TestClient::new();
    let alice_user = alice.signup_and_login().await;

    // Bob writes a post that mentions both Alice and himself. Only Alice may be
    // notified: authors are excluded from their own mention fan-out.
    let bob = TestClient::new();
    let bob_user = bob.signup_and_login().await;
    let board_id = board_id_by_slug(&bob, "qna").await;
    let title = format!("Mention {}", e2e::unique());
    let post_id = create_post(
        &bob,
        &board_id,
        &title,
        &format!(
            "hey @{} and @{} check this out\n",
            alice_user.handle, bob_user.handle
        ),
    )
    .await;

    let notification = wait_for_post_notification(&alice, "user_mentioned", &post_id).await;
    assert_eq!(notification["is_read"].as_bool(), Some(false));

    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["count"].as_u64(),
        Some(0),
        "self-mentions must not notify the author"
    );

    // Alice comments on Bob's post AND mentions him: Bob gets exactly one
    // user_mentioned notification — the mention supersedes the comment alert,
    // never both.
    create_comment(
        &alice,
        &post_id,
        &format!("thanks @{} for the post\n", bob_user.handle),
    )
    .await;

    wait_for_post_notification(&bob, "user_mentioned", &post_id).await;
    let comment_alerts = post_notifications(&bob, "board_comment_created", &post_id).await;
    assert!(
        comment_alerts.is_empty(),
        "a mentioned post author must not also get a board_comment_created duplicate"
    );
    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["count"].as_u64(), Some(1));
}

#[tokio::test]
async fn sec_007_notifications_are_scoped_per_user() {
    // Bob receives a notification; Alice (its trigger) and Carol (a bystander)
    // must not be able to read, mark, or delete it — the id must look nonexistent.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let board_id = board_id_by_slug(&bob, "general").await;
    let title = format!("Scope {}", e2e::unique());
    let post_id = create_post(&bob, &board_id, &title, "scoping probe post\n").await;

    let alice = TestClient::new();
    alice.signup_and_login().await;
    create_comment(&alice, &post_id, "trigger comment\n").await;

    let notification = wait_for_post_notification(&bob, "board_comment_created", &post_id).await;
    let notification_id = notification["id"].as_str().unwrap();

    let carol = TestClient::new();
    carol.signup_and_login().await;

    // Foreign inboxes never surface the row.
    for foreign in [&alice, &carol] {
        let resp = foreign.get("/v0/notifications/list?limit=50").await;
        let body = TestClient::json_ok(resp, StatusCode::OK).await;
        assert!(
            body["data"]
                .as_array()
                .unwrap()
                .iter()
                .all(|n| n["id"].as_str() != Some(notification_id)),
            "another user's notification must never appear in a foreign inbox"
        );

        // IDOR guard: marking or deleting a foreign notification is 404, never 204.
        let resp = foreign
            .post_json(
                "/v0/notifications/mark-as-read",
                &json!({ "notification_id": notification_id }),
            )
            .await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "a foreign notification must look nonexistent on mark-as-read"
        );
        let resp = foreign
            .post_json(
                "/v0/notifications/delete",
                &json!({ "notification_id": notification_id }),
            )
            .await;
        assert_eq!(
            resp.status(),
            StatusCode::NOT_FOUND,
            "a foreign notification must look nonexistent on delete"
        );
    }

    // The probes must not have mutated Bob's inbox.
    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["count"].as_u64(),
        Some(1),
        "foreign mark/delete probes must not touch the owner's notification"
    );

    // A foreign mark-all-as-read only affects the caller's own inbox.
    let resp = alice
        .post_json("/v0/notifications/mark-all-as-read", &json!({}))
        .await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let resp = bob.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["count"].as_u64(), Some(1));
}

#[tokio::test]
async fn action_preference_opt_out_suppresses_new_notifications() {
    // Carol opts out of comment alerts before Dave comments on her post.
    let carol = TestClient::new();
    carol.signup_and_login().await;
    let board_id = board_id_by_slug(&carol, "general").await;
    let title = format!("Optout {}", e2e::unique());
    let post_id = create_post(&carol, &board_id, &title, "opt-out target post\n").await;

    let resp = carol
        .post_json(
            "/v0/notifications/preferences/actions/update",
            &json!({
                "updates": [
                    { "action": "board_comment_created", "enabled": false }
                ]
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(body["preferences"].as_array().unwrap().iter().any(|pref| {
        pref["action"].as_str() == Some("board_comment_created")
            && pref["enabled"].as_bool() == Some(false)
    }));

    let dave = TestClient::new();
    dave.signup_and_login().await;
    create_comment(&dave, &post_id, "this should be suppressed\n").await;

    // Notifications are written during the comment request, so once it returned
    // 201 the (suppressed) fan-out has already run — no need to poll for absence.
    let suppressed = post_notifications(&carol, "board_comment_created", &post_id).await;
    assert!(
        suppressed.is_empty(),
        "a disabled action must not produce new notifications"
    );
    let resp = carol.get("/v0/notifications/unread/count").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["count"].as_u64(), Some(0));

    // Positive control: re-enable and the next comment lands in the inbox,
    // proving the earlier absence was the preference and not delivery lag.
    let resp = carol
        .post_json(
            "/v0/notifications/preferences/actions/update",
            &json!({
                "updates": [
                    { "action": "board_comment_created", "enabled": true }
                ]
            }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    create_comment(&dave, &post_id, "this one should arrive\n").await;
    let notification = wait_for_post_notification(&carol, "board_comment_created", &post_id).await;
    assert_eq!(notification["is_read"].as_bool(), Some(false));
}

#[tokio::test]
async fn notification_preferences_require_login_and_are_per_user() {
    let anon = TestClient::new();
    let resp = anon.get("/v0/notifications/preferences").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let resp = anon.get("/v0/notifications/preferences/actions").await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let alice = TestClient::new();
    alice.signup_and_login().await;
    let resp = alice.get("/v0/notifications/preferences").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["email_enabled"].as_bool(), Some(false));
    assert_eq!(body["push_enabled"].as_bool(), Some(false));

    let resp = alice
        .post_json(
            "/v0/notifications/preferences/update",
            &json!({ "email_enabled": true }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["email_enabled"].as_bool(), Some(true));
    assert_eq!(
        body["push_enabled"].as_bool(),
        Some(false),
        "omitted channel preferences should keep/default their existing value"
    );

    let resp = alice
        .post_json(
            "/v0/notifications/preferences/actions/update",
            &json!({
                "updates": [
                    { "action": "board_comment_created", "enabled": false },
                    { "action": "user_mentioned", "enabled": true }
                ]
            }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let preferences = body["preferences"].as_array().expect("action preferences");
    assert!(preferences.iter().any(|pref| {
        pref["action"].as_str() == Some("board_comment_created")
            && pref["enabled"].as_bool() == Some(false)
    }));
    assert!(preferences.iter().any(|pref| {
        pref["action"].as_str() == Some("user_mentioned") && pref["enabled"].as_bool() == Some(true)
    }));

    let resp = alice.get("/v0/notifications/preferences/actions").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let preferences = body["preferences"].as_array().expect("persisted actions");
    assert!(preferences.iter().any(|pref| {
        pref["action"].as_str() == Some("board_comment_created")
            && pref["enabled"].as_bool() == Some(false)
    }));

    let bob = TestClient::new();
    bob.signup_and_login().await;
    let resp = bob.get("/v0/notifications/preferences").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        body["email_enabled"].as_bool(),
        Some(false),
        "Alice's channel preference must not leak to Bob"
    );
    let resp = bob.get("/v0/notifications/preferences/actions").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["preferences"].as_array().unwrap().is_empty(),
        "Alice's action preferences must not leak to Bob"
    );
}
