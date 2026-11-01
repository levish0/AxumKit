use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn external_routes_share_key_budgets_and_preserve_error_and_secret_contracts() -> Result<()> {
    let fixture = Fixture::create(3, 4).await?;
    let second_app = bootstrap(&support::settings(&fixture.database)?).await?;
    let first = TestServer::new(kit::http::build_router(
        fixture.app.clone(),
        support::settings(&fixture.database)?.http,
    ));
    let second = TestServer::new(kit::http::build_router(
        second_app.clone(),
        support::settings(&fixture.database)?.http,
    ));
    let owner = format!("Bearer {}", fixture.owner_token);
    let key = format!("Bearer {}", fixture.credential);
    let lowercase_key = format!("bearer {}", fixture.credential);
    let unauthenticated = first.get("/api/v1/external/organization").await;
    unauthenticated.assert_status_unauthorized();
    assert_eq!(
        unauthenticated.headers()[header::WWW_AUTHENTICATE],
        "Bearer realm=\"api\""
    );
    let base = format!(
        "/api/v1/organizations/{}/api-clients/{}",
        fixture.organization_id, fixture.client_id
    );
    let issued = second
        .post(&format!("{base}/keys"))
        .add_header(header::AUTHORIZATION, &owner)
        .json(&json!({"name":"Rotated","expires_in_days":30}))
        .await;
    issued.assert_status(StatusCode::CREATED);
    assert_eq!(issued.headers()[header::CACHE_CONTROL], "no-store");
    let issued: Value = issued.json();
    let rotated = format!("Bearer {}", issued["credential"].as_str().unwrap());
    let stored = api_key::Entity::find_by_id(fixture.key_id)
        .one(&fixture.database.connection)
        .await?
        .unwrap();
    assert!(!stored.secret_hash.contains(&fixture.credential));
    let listed = first
        .get(&format!("{base}/keys"))
        .add_header(header::AUTHORIZATION, &owner)
        .await;
    listed.assert_status_ok();
    let listed = listed.json::<Value>().to_string();
    assert!(
        !listed.contains("secret_hash")
            && !listed.contains(&fixture.credential)
            && !listed.contains(issued["credential"].as_str().unwrap())
    );
    first
        .get("/api/v1/account")
        .add_header(header::AUTHORIZATION, &key)
        .await
        .assert_status_unauthorized();
    first
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &owner)
        .await
        .assert_status_unauthorized();
    first
        .get(&format!(
            "/api/v1/external/organization?api_key={}",
            fixture.credential
        ))
        .await
        .assert_status_unauthorized();
    first
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &key)
        .add_header(header::COOKIE, "session=ambiguous")
        .await
        .assert_status_bad_request();
    first
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &key)
        .add_header(header::AUTHORIZATION, &rotated)
        .await
        .assert_status_bad_request();
    let unsupported_head = first
        .method(axum::http::Method::HEAD, "/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &key)
        .await;
    unsupported_head.assert_status(StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(unsupported_head.headers()[header::ALLOW], "GET");
    stable_minute(&fixture.database).await?;
    for (server, credential, remaining) in [
        (&first, &lowercase_key, "2"),
        (&second, &rotated, "1"),
        (&first, &key, "0"),
    ] {
        let response = server
            .get("/api/v1/external/organization")
            .add_header(header::AUTHORIZATION, credential)
            .await;
        response.assert_status_ok();
        assert_eq!(response.headers()["x-ratelimit-remaining"], remaining);
        assert_eq!(
            response.json::<Value>()["id"],
            fixture.organization_id.to_string()
        );
        assert!(response.json::<Value>().get("owner_id").is_none());
    }
    let limited = second
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &rotated)
        .await;
    limited.assert_status_too_many_requests();
    assert_eq!(limited.json::<Value>()["error"]["code"], "api_rate_limited");
    let retry: u64 = limited.headers()[header::RETRY_AFTER].to_str()?.parse()?;
    assert!((1..=60).contains(&retry));
    assert_eq!(
        limited.json::<Value>()["request_id"].as_str().unwrap(),
        limited.headers()["x-request-id"]
    );
    fixture
        .app
        .api_access
        .set_quota_by_operator(fixture.client_id, 100, 3, TraceContext::default())
        .await?;
    let exhausted = first
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &rotated)
        .await;
    exhausted.assert_status_too_many_requests();
    assert_eq!(
        exhausted.json::<Value>()["error"]["code"],
        "api_quota_exceeded"
    );
    let month_retry: i64 = exhausted.headers()[header::RETRY_AFTER].to_str()?.parse()?;
    let current_quota = fixture
        .app
        .api_access
        .get_client(&fixture.actor, fixture.organization_id, fixture.client_id)
        .await?
        .quota;
    let expected_retry =
        (current_quota.month_resets_at - database_now(&fixture.database).await?).num_seconds();
    assert!((month_retry - expected_retry).abs() <= 2);
    first
        .delete(&format!("{base}/keys/{}", fixture.key_id))
        .add_header(header::AUTHORIZATION, &owner)
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
    second
        .get("/api/v1/external/organization")
        .add_header(header::AUTHORIZATION, &key)
        .await
        .assert_status_unauthorized();
    let current = first
        .get(&base)
        .add_header(header::AUTHORIZATION, &owner)
        .await;
    current.assert_status_ok();
    assert_eq!(current.json::<Value>()["quota"]["month_used"], 3);
    assert_eq!(
        api_usage::Entity::find()
            .count(&fixture.database.connection)
            .await?,
        3
    );
    second_app.close().await?;
    fixture.finish().await
}
