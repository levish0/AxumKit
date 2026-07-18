//! Board e2e tests. Run against the docker test stack.
//!
//! Policy references (`crates/server/src/permission/board.rs`):
//! - Board management (create/update/delete boards) is `board:manage` → admin-only.
//! - Posting requires login (RequiredSession); bans block writes.
//! - Post update/delete: content edits are owner-only; `board:moderate` may delete.
//! - Pin/unpin/lock/unlock and pin reordering are RBAC-gated (`board:pin_post`,
//!   `board:lock_post`); the `Mod` role holds them implicitly.

use e2e::TestClient;
use entity::common::Role;
use reqwest::StatusCode;
use serde_json::{Value, json};

/// Creates a fresh board as a brand-new admin and returns its id.
async fn create_board_as_admin() -> String {
    let admin = TestClient::new();
    let admin_user = admin.signup_and_login().await;
    e2e::grant_role(&admin_user.handle, Role::Admin).await;

    let slug = format!("e2e-{}", &e2e::unique()[..12]);
    let resp = admin
        .post_json(
            "/v0/board",
            &json!({ "slug": slug, "name": format!("E2E Board {slug}") }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;
    body["id"].as_str().expect("board id").to_string()
}

/// Creates a board (as admin) and a post authored by a fresh logged-in user.
/// Returns (board_id, author client, post_id).
async fn create_board_and_post() -> (String, TestClient, String) {
    let board_id = create_board_as_admin().await;
    let author = TestClient::new();
    author.signup_and_login().await;
    let resp = author
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": "post", "content": "body" }),
        )
        .await;
    let post = TestClient::json_ok(resp, StatusCode::CREATED).await;
    let post_id = post["id"].as_str().expect("post id").to_string();
    (board_id, author, post_id)
}

/// Creates a comment (or reply, when `parent_comment_id` is set) and returns its id.
async fn create_comment(
    client: &TestClient,
    post_id: &str,
    parent_comment_id: Option<&str>,
    content: &str,
) -> String {
    let mut payload = json!({ "post_id": post_id, "content": content });
    if let Some(pid) = parent_comment_id {
        payload["parent_comment_id"] = json!(pid);
    }
    let resp = client.post_json("/v0/board/comment", &payload).await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;
    body["id"].as_str().expect("comment id").to_string()
}

/// Reads the post's denormalized `comment_count`.
async fn post_comment_count(client: &TestClient, post_id: &str) -> i64 {
    let resp = client
        .get(&format!("/v0/board/post?post_id={post_id}"))
        .await;
    let post = TestClient::json_ok(resp, StatusCode::OK).await;
    post["comment_count"].as_i64().expect("comment_count")
}

/// Creates a post on `board_id` authored by `client` and returns its id.
async fn create_post(client: &TestClient, board_id: &str, title: &str) -> String {
    let resp = client
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": title, "content": "body" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::CREATED).await;
    body["id"].as_str().expect("post id").to_string()
}

/// A fresh logged-in client holding the `Mod` role (implicit pin/lock/moderate).
async fn board_moderator() -> TestClient {
    let client = TestClient::new();
    let user = client.signup_and_login().await;
    e2e::grant_role(&user.handle, Role::Mod).await;
    client
}

/// Pins a post as `moderator`, asserting the call succeeded.
async fn pin(moderator: &TestClient, post_id: &str) {
    let resp = moderator
        .post_json(
            "/v0/board/post/pin",
            &json!({ "post_id": post_id, "reason": "e2e pin" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "pin failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
}

/// Reads one page of a board listing as (pinned ids, paged post ids).
async fn list_page(
    client: &TestClient,
    board_id: &str,
    page: u32,
    page_size: u32,
) -> (Vec<String>, Vec<String>) {
    let resp = client
        .get(&format!(
            "/v0/board/post/list?board_id={board_id}&page={page}&page_size={page_size}"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let ids = |key: &str| -> Vec<String> {
        body[key]
            .as_array()
            .unwrap_or_else(|| panic!("`{key}` must be an array: {body}"))
            .iter()
            .map(|post| post["id"].as_str().expect("post id").to_string())
            .collect()
    };
    (ids("pinned"), ids("posts"))
}

#[tokio::test]
async fn seeded_boards_are_publicly_listed() {
    // The migration seeds three boards; the public listing must surface them.
    let anon = TestClient::new();
    let resp = anon.get("/v0/board/list?page=1&page_size=50").await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let slugs: Vec<&str> = body["boards"]
        .as_array()
        .expect("boards array")
        .iter()
        .map(|b| b["slug"].as_str().expect("slug"))
        .collect();
    for seeded in ["notice", "general", "qna"] {
        assert!(
            slugs.contains(&seeded),
            "seeded board `{seeded}` missing from listing: {slugs:?}"
        );
    }
}

#[tokio::test]
async fn board_management_is_admin_only() {
    let user = TestClient::new();
    user.signup_and_login().await;
    let resp = user
        .post_json(
            "/v0/board",
            &json!({ "slug": format!("nope-{}", &e2e::unique()[..8]), "name": "Not allowed" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "board creation by a non-admin must be 403"
    );

    // Admin creation works (covered by the helper) and the board is publicly visible.
    let board_id = create_board_as_admin().await;
    let anon = TestClient::new();
    let resp = anon.get(&format!("/v0/board?board_id={board_id}")).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn posting_requires_login() {
    let board_id = create_board_as_admin().await;

    let anon = TestClient::new();
    let resp = anon
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": "anon", "content": "anon post" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "board posting is login-only"
    );
}

#[tokio::test]
async fn post_update_and_delete_respect_ownership_and_moderation() {
    let board_id = create_board_as_admin().await;

    let alice = TestClient::new();
    alice.signup_and_login().await;
    let resp = alice
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": "alice post", "content": "original body" }),
        )
        .await;
    let post = TestClient::json_ok(resp, StatusCode::CREATED).await;
    let post_id = post["id"].as_str().unwrap().to_string();

    // Another regular user can neither update nor delete Alice's post.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let resp = bob
        .post_json(
            "/v0/board/post/update",
            &json!({ "post_id": post_id, "content": "hijacked" }),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "foreign post update");
    let resp = bob
        .post_json("/v0/board/post/delete", &json!({ "post_id": post_id }))
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN, "foreign post delete");

    // The owner can update their own post, and the content round-trips raw.
    let resp = alice
        .post_json(
            "/v0/board/post/update",
            &json!({ "post_id": post_id, "content": "edited **raw** by owner" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "owner update failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
    let resp = alice
        .get(&format!("/v0/board/post?post_id={post_id}"))
        .await;
    let post = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(
        post["content"].as_str().unwrap(),
        "edited **raw** by owner",
        "content must be stored and served exactly as written"
    );

    // A moderator can delete someone else's post (board moderation).
    let moderator = board_moderator().await;
    let resp = moderator
        .post_json("/v0/board/post/delete", &json!({ "post_id": post_id }))
        .await;
    assert!(
        resp.status().is_success(),
        "moderator delete failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
}

#[tokio::test]
async fn post_requires_title() {
    let board_id = create_board_as_admin().await;
    let user = TestClient::new();
    user.signup_and_login().await;

    // Title is required: omitting it fails JSON validation.
    let resp = user
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "content": "no title" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "missing title must be 400"
    );

    // A blank title is rejected by the not-blank validator.
    let resp = user
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": "   ", "content": "blank title" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "blank title must be 400"
    );
}

#[tokio::test]
async fn comment_create_and_list() {
    let (_board_id, author, post_id) = create_board_and_post().await;

    // Several top-level comments that all fit within one page.
    let first = create_comment(&author, &post_id, None, "first comment").await;
    create_comment(&author, &post_id, None, "second comment").await;
    create_comment(&author, &post_id, None, "third comment").await;

    // The public list returns the comments in oldest-first reading order.
    let anon = TestClient::new();
    let resp = anon
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&limit=10"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let comments = body["data"].as_array().expect("data");
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[0]["id"].as_str().unwrap(), first);
    assert!(comments[0]["parent_comment_id"].is_null());

    // Regression: when a full page of comments fits, the no-cursor page is
    // ascending (oldest-first), so a newer/older edge calc that assumes
    // descending order wrongly flips both flags to `true`.
    assert!(
        !body["has_newer"].as_bool().expect("has_newer"),
        "single full page must not advertise a newer page"
    );
    assert!(
        !body["has_older"].as_bool().expect("has_older"),
        "single full page must not advertise an older page"
    );

    // The post's denormalized counter reflects the new comments.
    assert_eq!(post_comment_count(&anon, &post_id).await, 3);
}

#[tokio::test]
async fn comments_paginate_oldest_first_regardless_of_direction() {
    let (_board_id, author, post_id) = create_board_and_post().await;
    let c1 = create_comment(&author, &post_id, None, "c1").await;
    let c2 = create_comment(&author, &post_id, None, "c2").await;
    let c3 = create_comment(&author, &post_id, None, "c3").await;

    let anon = TestClient::new();
    let ids = |body: &Value| -> Vec<String> {
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap().to_string())
            .collect()
    };

    // Older than c3: the earlier comments come back oldest-first, not reversed.
    let resp = anon
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&cursor_id={c3}&cursor_direction=Older&limit=10"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(ids(&body), vec![c1.clone(), c2.clone()]);
    assert!(!body["has_older"].as_bool().expect("has_older"));
    assert!(body["has_newer"].as_bool().expect("has_newer"));

    // Newer than c1: ascending as well, so direction never flips display order.
    let resp = anon
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&cursor_id={c1}&cursor_direction=Newer&limit=10"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(ids(&body), vec![c2.clone(), c3.clone()]);
    assert!(body["has_older"].as_bool().expect("has_older"));
    assert!(!body["has_newer"].as_bool().expect("has_newer"));
}

#[tokio::test]
async fn reply_depth_is_capped_at_two() {
    let (_board_id, author, post_id) = create_board_and_post().await;

    let a = create_comment(&author, &post_id, None, "A").await;
    let b = create_comment(&author, &post_id, Some(&a), "B").await;
    // Reply to a reply: must collapse onto the thread root A, not nest under B.
    let c = create_comment(&author, &post_id, Some(&b), "C").await;

    // Replies under A contain both B and C, each rooted at A.
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&parent_comment_id={a}&limit=10"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let replies = body["data"].as_array().expect("replies");
    let ids: Vec<&str> = replies.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&b.as_str()), "B should be a reply under A");
    assert!(
        ids.contains(&c.as_str()),
        "C (reply-to-reply) must collapse under root A"
    );
    for r in replies {
        assert_eq!(r["parent_comment_id"].as_str().unwrap(), a);
    }

    // Nothing nests under B (depth never exceeds 2).
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&parent_comment_id={b}&limit=10"
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(body["data"].as_array().unwrap().is_empty());
}

/// Deep-link anchor: `focus_comment_id` must return the page window containing
/// the target, for top-level comments and replies alike.
#[tokio::test]
async fn comment_list_focus_anchor_returns_containing_window() {
    let (_board_id, author, post_id) = create_board_and_post().await;

    let mut ids = Vec::new();
    for i in 0..7 {
        ids.push(create_comment(&author, &post_id, None, &format!("c{i}")).await);
    }

    // Focusing a middle comment centers the window on it.
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&focus_comment_id={}&limit=3",
            ids[3]
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let got: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(got, vec![ids[2].as_str(), ids[3].as_str(), ids[4].as_str()]);
    assert_eq!(body["has_newer"].as_bool(), Some(true));
    assert_eq!(body["has_older"].as_bool(), Some(true));

    // Focusing the oldest comment shifts the window forward to fill the page.
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&focus_comment_id={}&limit=3",
            ids[0]
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    let got: Vec<&str> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(got, vec![ids[0].as_str(), ids[1].as_str(), ids[2].as_str()]);
    assert_eq!(body["has_newer"].as_bool(), Some(true));
    assert_eq!(body["has_older"].as_bool(), Some(false));

    // A reply anchor only resolves within its own thread scope: the top-level
    // listing rejects it, the parent-scoped listing reveals it.
    let reply = create_comment(&author, &post_id, Some(&ids[0]), "reply").await;
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&focus_comment_id={reply}&limit=10"
        ))
        .await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&parent_comment_id={}&focus_comment_id={reply}&limit=10",
            ids[0]
        ))
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert!(
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"].as_str() == Some(reply.as_str())),
        "the reply window must contain the focused reply: {body}"
    );

    // Anchoring and cursor paging are mutually exclusive.
    let resp = author
        .get(&format!(
            "/v0/board/comment/list?post_id={post_id}&focus_comment_id={}&cursor_id={}&limit=10",
            ids[3], ids[0]
        ))
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn comment_count_tracks_creates_and_deletes() {
    let (_board_id, author, post_id) = create_board_and_post().await;

    let a = create_comment(&author, &post_id, None, "A").await;
    let _b = create_comment(&author, &post_id, Some(&a), "B").await;
    let reply = create_comment(&author, &post_id, Some(&a), "C").await;
    assert_eq!(post_comment_count(&author, &post_id).await, 3);

    // Deleting one reply drops the count by exactly 1.
    let resp = author
        .post_json("/v0/board/comment/delete", &json!({ "comment_id": reply }))
        .await;
    assert!(resp.status().is_success());
    assert_eq!(post_comment_count(&author, &post_id).await, 2);

    // Deleting the top-level comment cascades its remaining reply: 2 → 0.
    let resp = author
        .post_json("/v0/board/comment/delete", &json!({ "comment_id": a }))
        .await;
    assert!(resp.status().is_success());
    assert_eq!(post_comment_count(&author, &post_id).await, 0);
}

#[tokio::test]
async fn comment_update_and_delete_respect_ownership_and_moderation() {
    let (_board_id, alice, post_id) = create_board_and_post().await;
    let comment_id = create_comment(&alice, &post_id, None, "alice comment").await;

    // A foreign regular user can neither update nor delete Alice's comment.
    let bob = TestClient::new();
    bob.signup_and_login().await;
    let resp = bob
        .post_json(
            "/v0/board/comment/update",
            &json!({ "comment_id": comment_id, "content": "hijacked" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "foreign comment update"
    );
    let resp = bob
        .post_json(
            "/v0/board/comment/delete",
            &json!({ "comment_id": comment_id }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "foreign comment delete"
    );

    // The owner can edit their own comment.
    let resp = alice
        .post_json(
            "/v0/board/comment/update",
            &json!({ "comment_id": comment_id, "content": "edited by owner" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "owner update failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    // A moderator can delete someone else's comment (board moderation).
    let moderator = board_moderator().await;
    let resp = moderator
        .post_json(
            "/v0/board/comment/delete",
            &json!({ "comment_id": comment_id }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "moderator delete failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
}

/// Content edits are owner-only: a board moderator may sanction (delete/lock/
/// pin) but must not rewrite someone else's words. Regression for the
/// EditContent rule.
#[tokio::test]
async fn moderators_cannot_edit_others_content() {
    let board_id = create_board_as_admin().await;

    let alice = TestClient::new();
    alice.signup_and_login().await;
    let resp = alice
        .post_json(
            "/v0/board/post",
            &json!({ "board_id": board_id, "title": "alice post", "content": "alice's words" }),
        )
        .await;
    let post = TestClient::json_ok(resp, StatusCode::CREATED).await;
    let post_id = post["id"].as_str().unwrap().to_string();
    let comment_id = create_comment(&alice, &post_id, None, "alice's comment").await;

    let moderator = board_moderator().await;

    // A moderator cannot rewrite the post body...
    let resp = moderator
        .post_json(
            "/v0/board/post/update",
            &json!({ "post_id": post_id, "content": "rewritten by mod" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "moderator edit of another's post must be 403"
    );

    // ...nor the comment body...
    let resp = moderator
        .post_json(
            "/v0/board/comment/update",
            &json!({ "comment_id": comment_id, "content": "rewritten by mod" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "moderator edit of another's comment must be 403"
    );

    // ...while the sanction tools stay: the moderator can still delete it.
    let resp = moderator
        .post_json(
            "/v0/board/comment/delete",
            &json!({ "comment_id": comment_id }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "moderator delete failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );
}

/// Locking a post freezes its comment thread: new comments are rejected until
/// a moderator unlocks it.
#[tokio::test]
async fn locked_post_blocks_new_comments() {
    let (_board_id, author, post_id) = create_board_and_post().await;
    create_comment(&author, &post_id, None, "before lock").await;

    let moderator = board_moderator().await;
    let resp = moderator
        .post_json(
            "/v0/board/post/lock",
            &json!({ "post_id": post_id, "reason": "e2e lock" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["is_locked"].as_bool(), Some(true));

    // Even the post author cannot comment while the thread is locked.
    let resp = author
        .post_json(
            "/v0/board/comment",
            &json!({ "post_id": post_id, "content": "during lock" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "commenting on a locked post must be 403"
    );
    let body: Value = resp.json().await.expect("error body");
    assert_eq!(body["code"].as_str(), Some("board:post_locked"));

    // Locking is a moderation tool: the author cannot unlock their own post.
    let resp = author
        .post_json(
            "/v0/board/post/unlock",
            &json!({ "post_id": post_id, "reason": "nope" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "unlock by a non-moderator must be 403"
    );

    // Unlocking reopens the thread.
    let resp = moderator
        .post_json(
            "/v0/board/post/unlock",
            &json!({ "post_id": post_id, "reason": "e2e unlock" }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["is_locked"].as_bool(), Some(false));
    create_comment(&author, &post_id, None, "after unlock").await;
    assert_eq!(post_comment_count(&author, &post_id).await, 2);
}

/// A pin is board furniture, not a first-page decoration: it rides along on
/// every page and never eats a regular post's slot. Regression for the
/// `ORDER BY is_pinned DESC` + global OFFSET shape, under which pins were merely
/// the first rows of one paged result set — so page 2 never showed them, and a
/// board with 5 pins served only 5 regular posts on page 1.
#[tokio::test]
async fn pinned_posts_ride_along_on_every_page() {
    let board_id = create_board_as_admin().await;
    let author = TestClient::new();
    author.signup_and_login().await;

    let mut ids = Vec::new();
    for i in 0..5 {
        ids.push(create_post(&author, &board_id, &format!("post {i}")).await);
    }

    let moderator = board_moderator().await;
    pin(&moderator, &ids[0]).await;

    // Four unpinned posts remain, so page_size=2 yields two full pages.
    let anon = TestClient::new();
    for page in 1..=2 {
        let (pinned, posts) = list_page(&anon, &board_id, page, 2).await;
        assert_eq!(
            pinned,
            vec![ids[0].clone()],
            "the pin must be served on page {page}"
        );
        assert_eq!(
            posts.len(),
            2,
            "pins must not consume regular-post slots on page {page}"
        );
        assert!(
            !posts.contains(&ids[0]),
            "the pinned post must not be duplicated into the paged set on page {page}"
        );
    }
}

/// Pin order is moderator-controlled, not post age: a new pin lands on top, and
/// an explicit reorder rewrites the list to exactly what was sent.
#[tokio::test]
async fn pinned_posts_reorder_to_the_order_sent() {
    let board_id = create_board_as_admin().await;
    let author = TestClient::new();
    author.signup_and_login().await;

    let a = create_post(&author, &board_id, "a").await;
    let b = create_post(&author, &board_id, "b").await;
    let c = create_post(&author, &board_id, "c").await;

    let moderator = board_moderator().await;
    pin(&moderator, &a).await;
    pin(&moderator, &b).await;
    pin(&moderator, &c).await;

    // Each pin takes the top slot, so display order is the reverse of pin order.
    let anon = TestClient::new();
    let (pinned, _) = list_page(&anon, &board_id, 1, 10).await;
    assert_eq!(
        pinned,
        vec![c.clone(), b.clone(), a.clone()],
        "a newly pinned post must land on top"
    );

    let resp = moderator
        .post_json(
            "/v0/board/post/reorder-pins",
            &json!({ "board_id": board_id, "post_ids": [&a, &c, &b], "reason": "e2e reorder" }),
        )
        .await;
    assert!(
        resp.status().is_success(),
        "reorder failed: {} {}",
        resp.status(),
        resp.text().await.unwrap_or_default()
    );

    let (pinned, _) = list_page(&anon, &board_id, 1, 10).await;
    assert_eq!(pinned, vec![a, c, b], "reorder must apply the order sent");
}

/// Reordering is a moderation tool, gated exactly like pin itself — authoring a
/// pinned post grants no say over where it sits.
#[tokio::test]
async fn pin_reorder_requires_board_moderation() {
    let board_id = create_board_as_admin().await;
    let author = TestClient::new();
    author.signup_and_login().await;
    let post_id = create_post(&author, &board_id, "post").await;

    let moderator = board_moderator().await;
    pin(&moderator, &post_id).await;

    let resp = author
        .post_json(
            "/v0/board/post/reorder-pins",
            &json!({ "board_id": board_id, "post_ids": [&post_id], "reason": "nope" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "pin reorder by a non-moderator must be 403"
    );

    let anon = TestClient::new();
    let resp = anon
        .post_json(
            "/v0/board/post/reorder-pins",
            &json!({ "board_id": board_id, "post_ids": [&post_id], "reason": "nope" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "pin reorder is login-only"
    );
}

/// The payload must name exactly the board's current pin set. A moderator whose
/// list predates a concurrent pin/unpin is stale, and applying it would silently
/// drop or resurrect a pin they never saw — so it is rejected whole.
#[tokio::test]
async fn pin_reorder_rejects_a_stale_pin_set() {
    let board_id = create_board_as_admin().await;
    let author = TestClient::new();
    author.signup_and_login().await;
    let a = create_post(&author, &board_id, "a").await;
    let b = create_post(&author, &board_id, "b").await;
    let c = create_post(&author, &board_id, "c").await;

    let moderator = board_moderator().await;
    pin(&moderator, &a).await;
    pin(&moderator, &b).await;

    // Omitting a pinned post is a stale list, not a shorthand for unpinning it.
    let resp = moderator
        .post_json(
            "/v0/board/post/reorder-pins",
            &json!({ "board_id": board_id, "post_ids": [&a], "reason": "stale" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "an incomplete pin set must be 409, not a partial write"
    );
    let body: Value = resp.json().await.expect("error body");
    assert_eq!(body["code"].as_str(), Some("board:pin_set_mismatch"));

    // Naming an unpinned post is equally stale — reorder never pins.
    let resp = moderator
        .post_json(
            "/v0/board/post/reorder-pins",
            &json!({ "board_id": board_id, "post_ids": [&b, &a, &c], "reason": "stale" }),
        )
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::CONFLICT,
        "a pin set naming an unpinned post must be 409"
    );

    // Neither rejection wrote anything: b was pinned last, so it is still on top.
    let anon = TestClient::new();
    let (pinned, _) = list_page(&anon, &board_id, 1, 10).await;
    assert_eq!(pinned, vec![b, a], "a rejected reorder must not write");
}
