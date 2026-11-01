use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn presigned_originals_use_authorized_urls_and_preserve_bytes() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    let app = bootstrap(&settings).await?;
    app.files
        .initialize_integrations()
        .await
        .context("file integration provisioning")?;
    let (actor, credentials) = database.verified_account(&app.identity).await?;
    let (_, outsider) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Originals".into())
        .await?;
    let server = TestServer::new(build_router(app.clone(), settings.http));
    let authorization = format!("Bearer {}", credentials.token);
    let collection = format!("/api/v1/organizations/{}/files", organization.id);
    let bytes = Bytes::from_static(b"<svg onload=alert(1)>original bytes</svg>");
    let response = server.post(&collection).add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"filename":"proposal \"<image>.svg", "media_type":"image/svg+xml", "byte_size":bytes.len()})).await;
    response.assert_status(StatusCode::CREATED);
    let metadata: Value = response.json();
    let file_id = Uuid::parse_str(metadata["id"].as_str().unwrap())?;
    let item = format!("{collection}/{file_id}");
    let download_path = format!("{item}/download-url");
    server
        .post(&download_path)
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"method":"GET"}))
        .await
        .assert_status_not_found();
    server
        .get(&format!("{item}/content"))
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_not_found();
    let url_response = server
        .post(&format!("{item}/upload-url"))
        .add_header(header::AUTHORIZATION, &authorization)
        .await;
    url_response.assert_status_ok();
    assert_eq!(url_response.headers()[header::CACHE_CONTROL], "no-store");
    let transfer = transfer_from_json(url_response.json());
    assert!(!format!("{transfer:?}").contains("X-Amz"));
    let signed = url::Url::parse(&transfer.url)?
        .query_pairs()
        .find(|(key, _)| key == "X-Amz-SignedHeaders")
        .unwrap()
        .1
        .into_owned();
    for name in [
        "content-length",
        "content-type",
        "content-encoding",
        "if-none-match",
    ] {
        assert!(signed.split(';').any(|part| part == name));
    }
    assert!(
        transfer
            .headers
            .iter()
            .all(|header| header.name != "content-length")
    );
    let preflight = test_support::storage::request(reqwest::Method::OPTIONS, &transfer.url)?
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "PUT")
        .header(
            "Access-Control-Request-Headers",
            "content-type,content-encoding,content-disposition,cache-control,if-none-match",
        );
    let preflight = send_signed(preflight).await?;
    assert!(preflight.status().is_success());
    assert_eq!(
        preflight.headers()["access-control-allow-origin"],
        "http://localhost:3000"
    );
    assert!(
        send_signed(
            signed_request(&transfer)
                .header("content-type", "text/html")
                .body(bytes.clone())
        )
        .await?
        .status()
        .is_client_error()
    );
    assert!(
        send_signed(
            signed_request(&transfer)
                .header("x-amz-acl", "public-read")
                .body(bytes.clone())
        )
        .await?
        .status()
        .is_client_error()
    );
    assert!(
        send_signed(signed_request(&transfer).body(bytes.clone()))
            .await?
            .status()
            .is_success()
    );
    server
        .post(&format!("{item}/complete-upload"))
        .add_header(header::AUTHORIZATION, &authorization)
        .await
        .assert_status_ok();
    let ready = app.files.get_file(&actor, organization.id, file_id).await?;
    assert!(ready.content_hash.is_none());
    let (first, second) = tokio::join!(
        app.files
            .complete_upload(&actor, organization.id, file_id, TraceContext::default()),
        app.files
            .complete_upload(&actor, organization.id, file_id, TraceContext::default())
    );
    assert_eq!(first?.version, ready.version);
    assert_eq!(second?.version, ready.version);
    assert!(
        send_signed(signed_request(&transfer).body(bytes.clone()))
            .await?
            .status()
            .is_client_error()
    );
    server
        .post(&download_path)
        .add_header(header::AUTHORIZATION, format!("Bearer {}", outsider.token))
        .json(&json!({"method":"GET"}))
        .await
        .assert_status_not_found();
    let download_response = server
        .post(&download_path)
        .add_header(header::AUTHORIZATION, &authorization)
        .json(&json!({"method":"GET"}))
        .await;
    download_response.assert_status_ok();
    let download = transfer_from_json(download_response.json());
    let received = send_signed(signed_request(&download)).await?;
    assert_eq!(
        received.headers()[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    assert_eq!(received.headers()[header::CACHE_CONTROL], "no-store");
    assert!(
        received.headers()[header::CONTENT_DISPOSITION]
            .to_str()?
            .starts_with("attachment;")
    );
    assert_eq!(received.bytes().await?, bytes);
    for (range, expected) in [
        ("bytes=0-7", &bytes[..8]),
        ("bytes=-6", &bytes[bytes.len() - 6..]),
        ("bytes=2-", &bytes[2..]),
    ] {
        let received = send_signed(signed_request(&download).header("Range", range)).await?;
        assert_eq!(received.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(received.bytes().await?.as_ref(), expected);
    }
    let head = app
        .files
        .create_download_url(&actor, organization.id, file_id, DownloadMethod::Head)
        .await?;
    let response = send_signed(signed_request(&head)).await?;
    assert_eq!(
        response.headers()[header::CONTENT_LENGTH].to_str()?,
        bytes.len().to_string()
    );
    assert!(
        send_signed(test_support::storage::request(
            reqwest::Method::GET,
            &head.url
        )?)
        .await?
        .status()
        .is_client_error()
    );
    let mut short_settings = settings.files.clone();
    short_settings.download_url_seconds = 1;
    let short_lived = application::files::FileService::new(
        app.identity.clone(),
        app.organizations.clone(),
        short_settings,
    )?;
    let expired = short_lived
        .create_download_url(&actor, organization.id, file_id, DownloadMethod::Get)
        .await?;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        send_signed(signed_request(&expired))
            .await?
            .status()
            .is_client_error()
    );
    // A successful refresh uses the normal lifetime, independent of CI scheduling.
    let renewed = app
        .files
        .create_download_url(&actor, organization.id, file_id, DownloadMethod::Get)
        .await?;
    let received = send_signed(signed_request(&renewed)).await?;
    assert_eq!(received.status(), StatusCode::OK);
    assert_eq!(received.bytes().await?, bytes);
    drain_file_jobs(&app).await?;
    assert_eq!(
        app.files
            .search_files(&actor, organization.id, "proposal")
            .await?
            .len(),
        1
    );
    app.files
        .delete_file(&actor, organization.id, file_id, TraceContext::default())
        .await?;
    assert!(matches!(
        app.files
            .create_download_url(&actor, organization.id, file_id, DownloadMethod::Get)
            .await,
        Err(ApplicationError::NotFound)
    ));
    assert!(
        app.files
            .search_files(&actor, organization.id, "proposal")
            .await?
            .is_empty()
    );
    expire_file(&database, file_id).await?;
    drain_file_jobs(&app).await?;
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn exact_lengths_completion_and_empty_originals_are_enforced() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    let client = s3_client(&settings.files);
    let app = bootstrap(&settings).await?;
    app.files.initialize_integrations().await?;
    let (actor, _) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Lengths".into())
        .await?;
    let reservation = reserve(&app, &actor, organization.id, 2).await?;
    let transfer = app
        .files
        .create_upload_url(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default(),
        )
        .await?;
    assert!(matches!(
        app.files
            .complete_upload(
                &actor,
                organization.id,
                reservation.id,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::Conflict)
    ));
    for bytes in ["x", "too many bytes"] {
        assert!(
            send_signed(signed_request(&transfer).body(bytes))
                .await?
                .status()
                .is_client_error()
        );
    }
    client
        .put_object()
        .bucket("kit-files")
        .key(format!("originals/{}/{}", organization.id, reservation.id))
        .content_type("application/octet-stream")
        .content_encoding("identity")
        .body(Bytes::from_static(b"wrong").into())
        .send()
        .await?;
    assert!(matches!(
        app.files
            .complete_upload(
                &actor,
                organization.id,
                reservation.id,
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::InvalidInput(_))
    ));
    expire_file(&database, reservation.id).await?;
    app.files.reconcile_files().await?;
    drain_file_jobs(&app).await?;
    let empty = reserve(&app, &actor, organization.id, 0).await?;
    let ready = upload(&app, &actor, organization.id, empty.id, Bytes::new()).await?;
    assert_eq!(ready.byte_size, 0);
    let download = app
        .files
        .create_download_url(&actor, organization.id, empty.id, DownloadMethod::Get)
        .await?;
    assert!(
        send_signed(signed_request(&download))
            .await?
            .bytes()
            .await?
            .is_empty()
    );
    app.files
        .delete_file(&actor, organization.id, empty.id, TraceContext::default())
        .await?;
    expire_file(&database, empty.id).await?;
    drain_file_jobs(&app).await?;
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn deletion_retains_quota_and_cleanup_removes_late_presigned_uploads() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    settings.files.max_file_bytes = 4;
    settings.files.organization_quota_bytes = 4;
    let client = s3_client(&settings.files);
    let app = bootstrap(&settings).await?;
    app.files.initialize_integrations().await?;
    let (actor, _) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Delete race".into())
        .await?;
    let reservation = reserve(&app, &actor, organization.id, 4).await?;
    let transfer = app
        .files
        .create_upload_url(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default(),
        )
        .await?;
    app.files
        .delete_file(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default(),
        )
        .await?;
    drain_file_jobs(&app).await?;
    assert_eq!(
        file::Entity::find_by_id(reservation.id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        application::files::FileState::Deleting
    );
    let refused = reserve(&app, &actor, organization.id, 1)
        .await
        .err()
        .context("deleting file released its quota")?;
    assert!(matches!(
        refused.downcast_ref::<ApplicationError>(),
        Some(ApplicationError::FileQuotaExceeded)
    ));
    assert!(
        send_signed(signed_request(&transfer).body("late"))
            .await?
            .status()
            .is_success()
    );
    assert!(
        app.files
            .complete_upload(
                &actor,
                organization.id,
                reservation.id,
                TraceContext::default()
            )
            .await
            .is_err()
    );
    assert!(
        app.files
            .create_upload_url(
                &actor,
                organization.id,
                reservation.id,
                TraceContext::default()
            )
            .await
            .is_err()
    );
    expire_file(&database, reservation.id).await?;
    app.files.reconcile_files().await?;
    drain_file_jobs(&app).await?;
    let key = format!("originals/{}/{}", organization.id, reservation.id);
    assert!(
        client
            .head_object()
            .bucket("kit-files")
            .key(key)
            .send()
            .await
            .is_err()
    );
    assert_eq!(
        file::Entity::find_by_id(reservation.id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        application::files::FileState::Deleted
    );
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn concurrent_puts_and_completion_preserve_one_original_and_current_authority()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    let app = bootstrap(&settings).await?;
    app.files.initialize_integrations().await?;
    let (actor, credentials) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Immutable".into())
        .await?;
    let reservation = reserve(&app, &actor, organization.id, 4).await?;
    let transfer = app
        .files
        .create_upload_url(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default(),
        )
        .await?;
    let barrier = Arc::new(tokio::sync::Barrier::new(2));
    let body = |bytes: &'static [u8]| {
        let barrier = barrier.clone();
        reqwest::Body::wrap_stream(stream::unfold(
            (0, barrier),
            move |(step, barrier)| async move {
                if step == 0 {
                    Some((
                        Ok::<_, std::io::Error>(Bytes::copy_from_slice(&bytes[..2])),
                        (1, barrier),
                    ))
                } else if step == 1 {
                    barrier.wait().await;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Some((Ok(Bytes::copy_from_slice(&bytes[2..])), (2, barrier)))
                } else {
                    None
                }
            },
        ))
    };
    let (first, second) = tokio::join!(
        send_signed(
            signed_request(&transfer)
                .header("content-length", "4")
                .body(body(b"aaaa"))
        ),
        send_signed(
            signed_request(&transfer)
                .header("content-length", "4")
                .body(body(b"bbbb"))
        )
    );
    assert_eq!(
        usize::from(first?.status().is_success()) + usize::from(second?.status().is_success()),
        1
    );
    let (first, second) = tokio::join!(
        app.files.complete_upload(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default()
        ),
        app.files.complete_upload(
            &actor,
            organization.id,
            reservation.id,
            TraceContext::default()
        )
    );
    assert_eq!(first?.version, second?.version);
    let ready = app
        .files
        .get_file(&actor, organization.id, reservation.id)
        .await?;
    assert_eq!(ready.version, 3);
    let download = app
        .files
        .create_download_url(&actor, organization.id, ready.id, DownloadMethod::Get)
        .await?;
    let stored = send_signed(signed_request(&download))
        .await?
        .bytes()
        .await?;
    assert!(stored.as_ref() == b"aaaa" || stored.as_ref() == b"bbbb");
    membership::Entity::delete_many()
        .filter(membership::Column::OrganizationId.eq(organization.id))
        .filter(membership::Column::AccountId.eq(actor.account_id()))
        .exec(&database.connection)
        .await?;
    assert!(matches!(
        app.files
            .create_download_url(&actor, organization.id, ready.id, DownloadMethod::Get)
            .await,
        Err(ApplicationError::NotFound)
    ));
    assert!(
        app.files
            .complete_upload(&actor, organization.id, ready.id, TraceContext::default())
            .await
            .is_err()
    );
    assert!(
        send_signed(signed_request(&download))
            .await?
            .status()
            .is_success()
    );
    membership::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(organization.id),
        account_id: Set(actor.account_id()),
        role: Set(application::organizations::MembershipRole::Owner),
    }
    .insert(&database.connection)
    .await?;
    app.identity
        .revoke_session(&actor, actor.session_id(), TraceContext::default())
        .await?;
    assert!(
        app.identity
            .authenticate_session(&credentials.token)
            .await
            .is_err()
    );
    assert!(matches!(
        app.files
            .create_download_url(&actor, organization.id, ready.id, DownloadMethod::Get)
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    assert!(matches!(
        app.files
            .create_upload_url(&actor, organization.id, ready.id, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    assert!(matches!(
        app.files
            .complete_upload(&actor, organization.id, ready.id, TraceContext::default())
            .await,
        Err(ApplicationError::Unauthenticated)
    ));
    app.close().await?;
    database.finish().await
}
