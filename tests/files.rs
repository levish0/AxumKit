mod support;

#[path = "files/locking.rs"]
mod locking;
#[path = "files/presigned.rs"]
mod presigned;

use anyhow::Context;
use application::{
    ApplicationError,
    files::{
        DownloadMethod, FileSettings, FileSummary, PresignedTransfer, ReserveFile, S3Settings,
        SearchSettings, entities::file,
    },
    organizations::entities::membership,
};
use axum::http::{StatusCode, header};
use axum_test::TestServer;
use bytes::Bytes;
use futures_util::stream;
use jobs::TraceContext;
use kit::{
    bootstrap::{Application, bootstrap},
    http::build_router,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use test_support::TestDatabase;
use uuid::Uuid;

fn integration_settings() -> anyhow::Result<FileSettings> {
    let endpoint: url::Url = std::env::var("TEST_S3_ENDPOINT")?.parse()?;
    let search: url::Url = std::env::var("TEST_MEILISEARCH_URL")?.parse()?;
    for url in [&endpoint, &search] {
        anyhow::ensure!(
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
            "file integration tests require loopback services"
        );
    }
    Ok(FileSettings {
        storage: Some(S3Settings {
            endpoint,
            public_endpoint: None,
            bucket: "kit-files".into(),
            region: "us-east-1".into(),
            access_key: application::config::SecretString::new("kit-local-access"),
            secret_key: application::config::SecretString::new("kit-local-development-secret-only"),
            allow_http: true,
        }),
        search: Some(SearchSettings {
            endpoint: search,
            api_key: application::config::SecretString::new(
                "kit-local-meilisearch-key-development-only",
            ),
            index: format!("files_{}", Uuid::now_v7().simple()),
            allow_http: true,
        }),
        max_file_bytes: 1024 * 1024,
        organization_quota_bytes: 4 * 1024 * 1024,
        browser_origins: vec!["http://localhost:3000".into()],
        ..Default::default()
    })
}

fn signed_request(transfer: &PresignedTransfer) -> reqwest::RequestBuilder {
    let mut request = test_support::storage::request(
        reqwest::Method::from_bytes(transfer.method.as_str().as_bytes()).unwrap(),
        &transfer.url,
    )
    .expect("presigned test URL must use local storage");
    for header in &transfer.headers {
        request = request.header(&header.name, &header.value);
    }
    request
}

async fn send_signed(request: reqwest::RequestBuilder) -> anyhow::Result<reqwest::Response> {
    request
        .send()
        .await
        .map_err(|_| anyhow::anyhow!("direct storage request failed"))
}

async fn upload(
    app: &Application,
    actor: &application::identity::AuthenticatedSession,
    organization_id: Uuid,
    file_id: Uuid,
    bytes: impl Into<Bytes>,
) -> anyhow::Result<FileSummary> {
    let transfer = app
        .files
        .create_upload_url(actor, organization_id, file_id, TraceContext::default())
        .await?;
    let bytes = bytes.into();
    let response = send_signed(
        signed_request(&transfer)
            .header("content-length", bytes.len())
            .body(bytes),
    )
    .await?;
    let status = response.status();
    let error_code = if status.is_success() {
        String::new()
    } else {
        let body = response.text().await.unwrap_or_default();
        body.split_once("<Code>")
            .and_then(|(_, rest)| rest.split_once("</Code>"))
            .map(|(code, _)| code.to_owned())
            .unwrap_or_default()
    };
    anyhow::ensure!(
        status.is_success(),
        "direct PUT failed: {status}, {error_code}"
    );
    Ok(app
        .files
        .complete_upload(actor, organization_id, file_id, TraceContext::default())
        .await?)
}

async fn reserve(
    app: &Application,
    actor: &application::identity::AuthenticatedSession,
    organization_id: Uuid,
    size: u64,
) -> anyhow::Result<FileSummary> {
    Ok(app
        .files
        .reserve_file(
            actor,
            organization_id,
            ReserveFile {
                filename: "original.bin".into(),
                media_type: None,
                byte_size: size,
            },
            TraceContext::default(),
        )
        .await?)
}

fn transfer_from_json(value: Value) -> PresignedTransfer {
    PresignedTransfer {
        method: match value["method"].as_str().unwrap() {
            "PUT" => application::files::TransferMethod::Put,
            "HEAD" => application::files::TransferMethod::Head,
            "GET" => application::files::TransferMethod::Get,
            _ => panic!("unknown transfer method"),
        },
        url: value["url"].as_str().unwrap().into(),
        expires_at: value["expires_at"].as_str().unwrap().parse().unwrap(),
        headers: value["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|header| application::files::TransferHeader {
                name: header["name"].as_str().unwrap().into(),
                value: header["value"].as_str().unwrap().into(),
            })
            .collect(),
    }
}

async fn drain_file_jobs(app: &Application) -> anyhow::Result<()> {
    while let Some(claim) = app.queue.claim(30, 120).await? {
        app.files
            .execute_job(&claim)
            .await
            .map_err(|error| anyhow::anyhow!("file worker failed: {}", error.code))?;
        app.queue.finish(&claim, None).await?;
    }
    Ok(())
}

async fn expire_file(database: &TestDatabase, file_id: Uuid) -> anyhow::Result<()> {
    file::Entity::update_many()
        .col_expr(
            file::Column::UploadExpiresAt,
            sea_orm::sea_query::Expr::value(chrono::Utc::now() - chrono::Duration::hours(2)),
        )
        .filter(file::Column::Id.eq(file_id))
        .exec(&database.connection)
        .await?;
    Ok(())
}

fn s3_client(settings: &FileSettings) -> aws_sdk_s3::Client {
    let storage = settings.storage.as_ref().unwrap();
    aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(storage.endpoint.as_str())
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                storage.access_key.expose(),
                storage.secret_key.expose(),
                None,
                None,
                "file-tests",
            ))
            .force_path_style(true)
            .build(),
    )
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn concurrent_members_cannot_overreserve_organization_quota() -> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    settings.files.max_file_bytes = 3;
    settings.files.organization_quota_bytes = 3;
    settings.files.pending_file_limit = 2;
    let app = bootstrap(&settings).await?;
    let (owner, _) = database.verified_account(&app.identity).await?;
    let (member, _) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&owner, "Quota".into())
        .await?;
    membership::ActiveModel {
        id: Set(Uuid::now_v7()),
        organization_id: Set(organization.id),
        account_id: Set(member.account_id()),
        role: Set(application::organizations::MembershipRole::Member),
    }
    .insert(&database.connection)
    .await?;
    let reserve = |actor| {
        app.files.reserve_file(
            actor,
            organization.id,
            ReserveFile {
                filename: "small.bin".into(),
                media_type: None,
                byte_size: 2,
            },
            TraceContext::default(),
        )
    };
    let (first, second) = tokio::join!(reserve(&owner), reserve(&member));
    assert!(matches!(
        (&first, &second),
        (Ok(_), Err(ApplicationError::FileQuotaExceeded))
            | (Err(ApplicationError::FileQuotaExceeded), Ok(_))
    ));
    let winner = if first.is_ok() { &owner } else { &member };
    app.files
        .reserve_file(
            winner,
            organization.id,
            ReserveFile {
                filename: "empty.bin".into(),
                media_type: None,
                byte_size: 0,
            },
            TraceContext::default(),
        )
        .await?;
    assert!(matches!(
        app.files
            .reserve_file(
                winner,
                organization.id,
                ReserveFile {
                    filename: "another-empty.bin".into(),
                    media_type: None,
                    byte_size: 0
                },
                TraceContext::default()
            )
            .await,
        Err(ApplicationError::FileQuotaExceeded)
    ));
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn zero_byte_reservations_are_rate_limited_and_old_confirmed_tombstones_are_purged()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    settings.files.reservations_per_minute = 1;
    let app = bootstrap(&settings).await?;
    app.files.initialize_integrations().await?;
    let (actor, _) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Bounded metadata".into())
        .await?;
    let input = || ReserveFile {
        filename: "empty.bin".into(),
        media_type: None,
        byte_size: 0,
    };
    let original = app
        .files
        .reserve_file(&actor, organization.id, input(), TraceContext::default())
        .await?;
    assert!(matches!(
        app.files
            .reserve_file(&actor, organization.id, input(), TraceContext::default())
            .await,
        Err(ApplicationError::RateLimited)
    ));
    upload(&app, &actor, organization.id, original.id, Bytes::new()).await?;
    drain_file_jobs(&app).await?;
    let mut disabled_settings = settings.files.clone();
    disabled_settings.search = None;
    let search_disabled = application::files::FileService::new(
        app.identity.clone(),
        app.organizations.clone(),
        disabled_settings,
    )?;
    search_disabled
        .delete_file(
            &actor,
            organization.id,
            original.id,
            TraceContext::default(),
        )
        .await?;
    expire_file(&database, original.id).await?;
    while let Some(claim) = app.queue.claim(30, 120).await? {
        search_disabled
            .execute_job(&claim)
            .await
            .map_err(|error| anyhow::anyhow!("file deletion failed: {}", error.code))?;
        app.queue.finish(&claim, None).await?;
    }
    let old = chrono::Utc::now() - chrono::Duration::days(40);
    file::Entity::update_many()
        .col_expr(
            file::Column::CreatedAt,
            sea_orm::sea_query::Expr::value(old),
        )
        .col_expr(
            file::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(old),
        )
        .col_expr(
            file::Column::UploadExpiresAt,
            sea_orm::sea_query::Expr::value(old),
        )
        .filter(file::Column::Id.eq(original.id))
        .exec(&database.connection)
        .await?;
    search_disabled.reconcile_files().await?;
    assert!(
        file::Entity::find_by_id(original.id)
            .one(&database.connection)
            .await?
            .unwrap()
            .search_document_may_exist
    );
    app.files.repair_search().await?;
    drain_file_jobs(&app).await?;
    app.files.reconcile_files().await?;
    assert!(
        file::Entity::find_by_id(original.id)
            .one(&database.connection)
            .await?
            .is_none()
    );
    app.close().await?;
    database.finish().await
}

#[tokio::test]
#[ignore = "requires PostgreSQL, SeaweedFS and Meilisearch; run just test-files"]
async fn cleanup_prioritizes_unscheduled_uploads_and_advances_between_hourly_batches()
-> anyhow::Result<()> {
    let database = TestDatabase::create().await?;
    let mut settings = support::settings(&database)?;
    settings.files = integration_settings()?;
    settings.files.search = None;
    let app = bootstrap(&settings).await?;
    let (actor, _) = database.verified_account(&app.identity).await?;
    let organization = app
        .organizations
        .create_organization(&actor, "Cleanup fairness".into())
        .await?;
    let now = chrono::Utc::now();
    let rows: Vec<_> = (0..101)
        .map(|_| {
            let id = Uuid::now_v7();
            file::ActiveModel {
                id: Set(id),
                organization_id: Set(organization.id),
                uploader_id: Set(actor.account_id()),
                object_key: Set(format!("originals/{}/{}", organization.id, id)),
                filename: Set("old.bin".into()),
                byte_size: Set(0),
                state: Set(application::files::FileState::Deleted),
                upload_expires_at: Set(now - chrono::Duration::days(2)),
                updated_at: Set(now - chrono::Duration::days(1)),
                created_at: Set(now - chrono::Duration::days(1)),
                cleanup_scheduled_at: Set(Some(now - chrono::Duration::hours(2))),
                ..Default::default()
            }
        })
        .collect();
    file::Entity::insert_many(rows)
        .exec(&database.connection)
        .await?;
    let expired = app
        .files
        .reserve_file(
            &actor,
            organization.id,
            ReserveFile {
                filename: "expired.bin".into(),
                media_type: None,
                byte_size: 1,
            },
            TraceContext::default(),
        )
        .await?;
    expire_file(&database, expired.id).await?;
    assert_eq!(app.files.reconcile_files().await?, 100);
    assert_eq!(
        file::Entity::find_by_id(expired.id)
            .one(&database.connection)
            .await?
            .unwrap()
            .state,
        application::files::FileState::Deleting
    );
    let old_schedule = now - chrono::Duration::minutes(90);
    use sea_orm::PaginatorTrait;
    assert_eq!(
        file::Entity::find()
            .filter(file::Column::CleanupScheduledAt.lte(old_schedule))
            .count(&database.connection)
            .await?,
        2
    );
    file::Entity::update_many()
        .col_expr(
            file::Column::CleanupScheduledAt,
            sea_orm::sea_query::Expr::value(Some(now - chrono::Duration::minutes(70))),
        )
        .filter(file::Column::CleanupScheduledAt.gt(now - chrono::Duration::minutes(5)))
        .exec(&database.connection)
        .await?;
    assert_eq!(app.files.reconcile_files().await?, 100);
    assert_eq!(
        file::Entity::find()
            .filter(file::Column::CleanupScheduledAt.lte(old_schedule))
            .count(&database.connection)
            .await?,
        0
    );
    app.close().await?;
    database.finish().await
}
