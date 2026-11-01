use super::*;
use reqwest::header;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-process"]
async fn external_keys_and_operator_allocations_are_shared_across_api_processes_and_restarts()
-> Result<()> {
    let database = TestDatabase::create().await?;
    let environment = ProcessEnvironment::new(&database)?;
    let settings = kit::config::Settings::from_values(&environment.values)?;
    let app = kit::bootstrap::bootstrap(&settings).await?;
    let (actor, credentials) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Process developer organization".into())
        .await?;
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let (mut first, first_url, _) = environment.api(&client, false).await?;
    let (mut second, second_url, _) = environment.api(&client, false).await?;
    let management_path = format!("/api/v1/organizations/{}/api-clients", organization.id);
    let created: Value = client
        .post(format!("{first_url}{management_path}"))
        .bearer_auth(&credentials.token)
        .json(&json!({"name":"Process partner","scopes":["organizations:read"]}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let client_id = created["id"].as_str().context("client ID missing")?;
    let client_path = format!("{management_path}/{client_id}");
    let issue = |base: String, name: &'static str| {
        let client = client.clone();
        let token = credentials.token.clone();
        let path = format!("{client_path}/keys");
        async move {
            Ok::<Value, anyhow::Error>(
                client
                    .post(format!("{base}{path}"))
                    .bearer_auth(token)
                    .json(&json!({"name":name,"expires_in_days":30}))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?,
            )
        }
    };
    let first_key = issue(first_url.clone(), "Primary").await?;
    let second_key = issue(second_url.clone(), "Rotation").await?;
    let primary = first_key["credential"]
        .as_str()
        .context("primary key missing")?;
    let rotated = second_key["credential"]
        .as_str()
        .context("rotated key missing")?;
    assert_eq!(
        client
            .get(format!("{second_url}/api/v1/external/organization"))
            .bearer_auth(primary)
            .send()
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    let mut operator = environment.start_with_arguments(
        env!("CARGO_BIN_EXE_manage"),
        &["set-api-client-quota", client_id, "100", "2"],
        None,
    )?;
    ensure!(
        tokio::time::timeout(Duration::from_secs(15), operator.child.wait())
            .await??
            .success(),
        "operator allocation failed"
    );
    for (base, key) in [(&first_url, primary), (&second_url, rotated)] {
        let response = client
            .get(format!("{base}/api/v1/external/organization"))
            .bearer_auth(key)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            response.json::<Value>().await?["id"],
            organization.id.to_string()
        );
    }
    let exhausted = client
        .get(format!("{first_url}/api/v1/external/organization"))
        .bearer_auth(primary)
        .send()
        .await?;
    assert_eq!(exhausted.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        exhausted.json::<Value>().await?["error"]["code"],
        "api_quota_exceeded"
    );
    first.stop().await?;
    let (mut replacement, replacement_url, _) = environment.api(&client, false).await?;
    assert_eq!(
        client
            .get(format!("{replacement_url}/api/v1/external/organization"))
            .bearer_auth(rotated)
            .send()
            .await?
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    client
        .delete(format!(
            "{second_url}{client_path}/keys/{}",
            first_key["key"]["id"].as_str().unwrap()
        ))
        .bearer_auth(&credentials.token)
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(
        client
            .get(format!("{replacement_url}/api/v1/external/organization"))
            .bearer_auth(primary)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    for process in [&mut second, &mut replacement] {
        process.stop().await?;
    }
    for process in [&first, &second, &replacement, &operator] {
        process.assert_secrets_absent(&[primary, rotated, &credentials.token, TEST_PASSWORD])?;
    }
    app.close().await?;
    database.finish().await
}
