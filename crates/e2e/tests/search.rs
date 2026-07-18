//! Search e2e tests. Run via `just e2e`.
//!
//! Policy references:
//! - User search is public and goes through MeiliSearch; the worker indexes
//!   users asynchronously after signup verification.
//! - Profile updates (handle/display_name/bio) re-queue the user for indexing,
//!   so the changed fields become searchable.
//! - Account deletion removes the user document from the search index, so a
//!   deleted account is no longer discoverable.

use std::time::Duration;

use e2e::TestClient;
use reqwest::StatusCode;
use serde_json::{Value, json};

/// Polls user search until `handle`'s presence matches `expect_present`.
/// Indexing is asynchronous (NATS → worker → MeiliSearch), so the budget is generous.
async fn wait_for_user_search_presence(query: &str, handle: &str, expect_present: bool) {
    let client = TestClient::new();
    for _ in 0..240 {
        let resp = client
            .get_q(
                "/v0/search/users",
                &[("query", query), ("page", "1"), ("page_size", "20")],
            )
            .await;
        if resp.status() == StatusCode::OK
            && let Ok(body) = resp.json::<Value>().await
        {
            let present = body["users"]
                .as_array()
                .is_some_and(|users| users.iter().any(|u| u["handle"].as_str() == Some(handle)));
            if present == expect_present {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    panic!(
        "timed out waiting for user {handle} to be {} in search results for {query:?}",
        if expect_present { "present" } else { "absent" }
    );
}

/// Fetches the search hit for `handle` (which must already be indexed).
async fn user_search_hit(query: &str, handle: &str) -> Value {
    let client = TestClient::new();
    let resp = client
        .get_q(
            "/v0/search/users",
            &[("query", query), ("page", "1"), ("page_size", "20")],
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    body["users"]
        .as_array()
        .and_then(|users| {
            users
                .iter()
                .find(|u| u["handle"].as_str() == Some(handle))
                .cloned()
        })
        .unwrap_or_else(|| panic!("no search hit for user {handle} with query {query:?}"))
}

#[tokio::test]
async fn user_search_indexes_new_signups() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // The new user becomes searchable once the worker indexes it.
    wait_for_user_search_presence(&user.handle, &user.handle, true).await;

    // The hit carries the public profile fields, not the email.
    let hit = user_search_hit(&user.handle, &user.handle).await;
    assert!(hit["id"].as_str().is_some(), "hit should carry the user id");
    assert_eq!(hit["display_name"].as_str(), Some("E2E Test User"));
    assert!(hit.get("email").is_none(), "email must not be indexed");
}

#[tokio::test]
async fn profile_update_reindexes_user_search() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Baseline: indexed under the signup handle.
    wait_for_user_search_presence(&user.handle, &user.handle, true).await;

    // A nonsense token unique to this run keeps the query unambiguous.
    let token = format!("rq{}", &e2e::unique()[..10]);
    let display_name = format!("Renamed {token}");
    let bio = format!("bio mentioning {token}");
    let resp = client
        .patch_json(
            "/v0/user/me",
            &json!({ "display_name": display_name, "bio": bio }),
        )
        .await;
    let body = TestClient::json_ok(resp, StatusCode::OK).await;
    assert_eq!(body["display_name"].as_str(), Some(display_name.as_str()));

    // The reindex must make the new display_name/bio token searchable.
    wait_for_user_search_presence(&token, &user.handle, true).await;
    let hit = user_search_hit(&token, &user.handle).await;
    assert_eq!(hit["display_name"].as_str(), Some(display_name.as_str()));
    assert_eq!(hit["bio"].as_str(), Some(bio.as_str()));
}

#[tokio::test]
async fn account_deletion_removes_user_from_search() {
    let client = TestClient::new();
    let user = client.signup_and_login().await;

    // Baseline: indexed and publicly searchable.
    wait_for_user_search_presence(&user.handle, &user.handle, true).await;

    // Self-service deletion re-authenticates with the account password and
    // completes inline (204) for password accounts.
    let resp = client
        .delete_json("/v0/user/me", &json!({ "password": user.password }))
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "account deletion should succeed"
    );

    // The deleted account must disappear from the public search index.
    wait_for_user_search_presence(&user.handle, &user.handle, false).await;
}
