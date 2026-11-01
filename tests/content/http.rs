use super::fixture::Fixture;
use application::posts::PostPublication;
use axum::http::{StatusCode, header};
use jobs::TraceContext;
use serde_json::{Value, json};

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn versioned_routes_validate_contracts_and_keep_sessions_separate_from_api_keys()
-> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let server = f.server();
    let writer = format!("Bearer {}", f.writer_credentials.token);
    let path = format!("/api/v1/organizations/{}/posts", f.organization_id);
    server.get(&path).await.assert_status_unauthorized();
    server.post(&path).add_header(header::AUTHORIZATION, &writer)
        .json(&json!({"title":"Invalid state","body":"Text","state":"deleted","attachment_ids":[],"expected_version":0}))
        .await.assert_status_bad_request();
    server
        .post("/api/v1/account/push-devices")
        .add_header(header::AUTHORIZATION, &writer)
        .json(&json!({"token":"test-only-device-token-at-least-20","platform":"unknown"}))
        .await
        .assert_status_bad_request();
    server.post(&path).add_header(header::AUTHORIZATION,&writer).json(&json!({"title":"Valid","body":"Text","state":"published","attachment_ids":[],"expected_version":0,"unknown":true})).await.assert_status_bad_request();
    server.post(&path).add_header(header::AUTHORIZATION,&writer).json(&json!({"title":" ","body":"Text","state":"published","attachment_ids":[],"expected_version":0})).await.assert_status_bad_request();
    server.post(&path).add_header(header::AUTHORIZATION,&writer).json(&json!({"title":"Large","body":"x".repeat(17000),"state":"published","attachment_ids":[],"expected_version":0})).await.assert_status(StatusCode::PAYLOAD_TOO_LARGE);
    let response=server.post(&path).add_header(header::AUTHORIZATION,&writer).json(&json!({"title":"Example","body":"Hello","state":"published","attachment_ids":[],"expected_version":0})).await;
    response.assert_status(StatusCode::CREATED);
    let post: Value = response.json();
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert!(response.headers().contains_key("x-request-id"));
    let note_path = format!("{path}/{}/comments", post["id"].as_str().unwrap());
    server
        .post(&note_path)
        .add_header(header::AUTHORIZATION, &writer)
        .json(&json!({"body":"Own reply"}))
        .await
        .assert_status(StatusCode::CREATED);
    let notes: Value = server
        .get("/api/v1/notifications/unread-count")
        .add_header(header::AUTHORIZATION, &writer)
        .await
        .json();
    assert_eq!(notes["unread_count"], 0);
    let owner = format!("Bearer {}", f.owner_credentials.token);
    server
        .post(&note_path)
        .add_header(header::AUTHORIZATION, &owner)
        .json(&json!({"body":"Owner response"}))
        .await
        .assert_status(StatusCode::CREATED);
    let notes: Value = server
        .get("/api/v1/notifications/unread-count")
        .add_header(header::AUTHORIZATION, &writer)
        .await
        .json();
    assert_eq!(notes["unread_count"], 1);
    server
        .get(&format!("{path}/search"))
        .add_query_param("q", "hello")
        .add_header(header::AUTHORIZATION, &writer)
        .await
        .assert_status(StatusCode::SERVICE_UNAVAILABLE);
    server
        .post("/api/v1/account/push-devices")
        .add_header(header::AUTHORIZATION, &writer)
        .json(&json!({"token":"test-only-device-token-at-least-20","platform":"web"}))
        .await
        .assert_status(StatusCode::SERVICE_UNAVAILABLE);
    server
        .put("/api/v1/account/profile")
        .add_header(header::AUTHORIZATION, &writer)
        .json(&json!({"display_name":"Writer","bio":"Public profile","expected_version":0}))
        .await
        .assert_status_ok();
    let users: Value = server
        .get(&format!(
            "/api/v1/organizations/{}/users",
            f.organization_id
        ))
        .add_header(header::AUTHORIZATION, &writer)
        .await
        .json();
    assert_eq!(users["items"].as_array().unwrap().len(), 1);
    assert!(users["items"][0].get("email").is_none());
    let outsider = format!("Bearer {}", f.outsider_credentials.token);
    server
        .get(&path)
        .add_header(header::AUTHORIZATION, &outsider)
        .await
        .assert_status_not_found();
    f.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-content"]
async fn external_posts_read_only_published_scope_and_charge_shared_quotas() -> anyhow::Result<()> {
    let f = Fixture::new(false, false).await?;
    let visible = f
        .post(&f.owner, "Partner visible", PostPublication::Published)
        .await?;
    f.post(&f.owner, "Partner secret", PostPublication::Draft)
        .await?;
    let client = f
        .app
        .api_access
        .create_client(
            &f.owner,
            f.organization_id,
            "Partner".into(),
            vec![application::api_access::ApiScope::PostsRead],
            TraceContext::default(),
        )
        .await?;
    f.app
        .api_access
        .set_quota_by_operator(client.id, 2, 10, TraceContext::default())
        .await?;
    let key = f
        .app
        .api_access
        .issue_key(
            &f.owner,
            f.organization_id,
            client.id,
            "Primary".into(),
            1,
            TraceContext::default(),
        )
        .await?;
    let server = f.server();
    let credential = format!("Bearer {}", key.credential.expose());
    let response = server
        .get("/api/v1/external/posts")
        .add_header(header::AUTHORIZATION, &credential)
        .await;
    response.assert_status_ok();
    assert_eq!(response.headers()["x-ratelimit-remaining"], "1");
    let data: Value = response.json();
    assert_eq!(data["items"].as_array().unwrap().len(), 1);
    assert_eq!(data["items"][0]["id"], visible.id.to_string());
    server
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &credential)
        .await
        .assert_status_forbidden();
    server
        .get(&format!(
            "/api/v1/organizations/{}/posts",
            f.organization_id
        ))
        .add_header(header::AUTHORIZATION, &credential)
        .await
        .assert_status_unauthorized();
    server
        .method(axum::http::Method::HEAD, "/api/v1/external/posts")
        .add_header(header::AUTHORIZATION, &credential)
        .await
        .assert_status(StatusCode::METHOD_NOT_ALLOWED);
    server
        .get("/api/v1/external/posts")
        .add_header(header::AUTHORIZATION, &credential)
        .await
        .assert_status_ok();
    server
        .get("/api/v1/external/posts")
        .add_header(header::AUTHORIZATION, &credential)
        .await
        .assert_status(StatusCode::TOO_MANY_REQUESTS);
    f.finish().await
}
