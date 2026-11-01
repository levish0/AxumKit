use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL; run just test-api"]
async fn multiple_providers_bind_callbacks_and_share_sessions_across_api_instances() -> Result<()> {
    use axum::http::header;
    use axum_test::TestServer;
    use kit::{bootstrap::bootstrap, http::build_router};

    let database = TestDatabase::create().await?;
    let first_provider = MockProvider::start().await?;
    let second_provider = MockProvider::start().await?;
    let mut settings = support::settings(&database)?;
    settings.oidc = [&first_provider, &second_provider]
        .into_iter()
        .zip(["first", "second"])
        .map(|(provider, name)| {
            let mut settings = provider.settings();
            settings.name = name.into();
            settings.redirect_url =
                format!("http://localhost:3000/api/v1/auth/oidc/{name}/callback")
                    .parse()
                    .unwrap();
            settings
        })
        .collect();
    let first_app = bootstrap(&settings).await?;
    let second_app = bootstrap(&settings).await?;
    let first_api = TestServer::new(build_router(first_app.clone(), settings.http.clone()));
    let second_api = TestServer::new(build_router(second_app.clone(), settings.http));
    let mut account_ids = Vec::new();

    for (name, other_name, provider) in [
        ("first", "second", &first_provider),
        ("second", "first", &second_provider),
    ] {
        let response = first_api
            .post(&format!("/api/v1/auth/oidc/{name}"))
            .add_header(header::ORIGIN, "http://localhost:3000")
            .await;
        response.assert_status_ok();
        let cookie = response.headers()[header::SET_COOKIE]
            .to_str()?
            .split(';')
            .next()
            .context("OIDC binding cookie missing")?
            .to_owned();
        let redirect = AuthorizationRedirect {
            url: response.json::<serde_json::Value>()["authorization_url"]
                .as_str()
                .context("authorization URL missing")?
                .parse()?,
            browser_binding: cookie
                .split_once('=')
                .context("OIDC binding missing")?
                .1
                .into(),
        };
        let (state, code) = provider.authorize(
            &redirect,
            &format!("{name}@example.test"),
            "same-subject-different-issuer",
            false,
            false,
        )?;
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("state", &state)
            .append_pair("code", &code)
            .finish();
        let mismatched = second_api
            .get(&format!("/api/v1/auth/oidc/{other_name}/callback?{query}"))
            .add_header(header::COOKIE, &cookie)
            .await;
        mismatched.assert_status_unauthorized();
        let error = mismatched.json::<serde_json::Value>();
        assert_eq!(error["error"]["code"], "unauthenticated");
        assert_eq!(
            error["request_id"].as_str().unwrap(),
            mismatched.headers()["x-request-id"]
        );

        let callback = second_api
            .get(&format!("/api/v1/auth/oidc/{name}/callback?{query}"))
            .add_header(header::COOKIE, &cookie)
            .await;
        callback.assert_status_ok();
        assert_eq!(
            callback.json::<serde_json::Value>()["status"],
            "authenticated"
        );
        let session_cookie = callback
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|header| header.to_str().ok())
            .find(|header| header.starts_with("session="))
            .context("session cookie missing")?
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        let account = first_api
            .get("/api/v1/account")
            .add_header(header::COOKIE, session_cookie)
            .await;
        account.assert_status_ok();
        account_ids.push(account.json::<serde_json::Value>()["id"].clone());
        first_api
            .get(&format!("/api/v1/auth/oidc/{name}/callback?{query}"))
            .add_header(header::COOKIE, cookie)
            .await
            .assert_status_unauthorized();
    }
    assert_ne!(account_ids[0], account_ids[1]);
    let response = first_api
        .post("/api/v1/auth/oidc/first")
        .add_header(header::ORIGIN, "http://localhost:3000")
        .await;
    response.assert_status_ok();
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()?
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let redirect = AuthorizationRedirect {
        url: response.json::<serde_json::Value>()["authorization_url"]
            .as_str()
            .unwrap()
            .parse()?,
        browser_binding: cookie.split_once('=').unwrap().1.into(),
    };
    let (state, code) = first_provider.authorize(
        &redirect,
        "first@example.test",
        "same-subject-different-issuer",
        false,
        true,
    )?;
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("state", &state)
        .append_pair("code", &code)
        .finish();
    let unavailable = second_api
        .get(&format!("/api/v1/auth/oidc/first/callback?{query}"))
        .add_header(header::COOKIE, cookie)
        .await;
    unavailable.assert_status_service_unavailable();
    let error = unavailable.json::<serde_json::Value>();
    assert_eq!(error["error"]["code"], "provider_unavailable");
    assert_eq!(
        error["request_id"].as_str().unwrap(),
        unavailable.headers()["x-request-id"]
    );
    let response = first_api
        .post("/api/v1/auth/oidc/missing")
        .add_header(header::ORIGIN, "http://localhost:3000")
        .await;
    response.assert_status_not_found();
    assert_eq!(
        response.json::<serde_json::Value>()["error"]["code"],
        "not_found"
    );
    first_app.close().await?;
    second_app.close().await?;
    database.finish().await
}
