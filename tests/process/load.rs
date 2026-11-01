use super::{ProcessEnvironment, RunningProcess};
use anyhow::{Context, Result, ensure};
use application::{
    api_access::{ApiAccessService, entities::api_usage},
    files::entities::file,
    identity::LoginOutcome,
    organizations::OrganizationService,
};
use jobs::{
    TraceContext,
    entities::{job, job_attempt},
};
use reqwest::Client;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, sea_query::Expr};
use serde::Serialize;
use std::{collections::BTreeMap, time::Duration};
use test_support::{TEST_PASSWORD, TestDatabase};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Serialize)]
struct LoadAccount {
    account_id: Uuid,
    token: String,
    commenter_token: String,
    revocable_token: String,
    revocable_session_id: Uuid,
    organization_ids: [Uuid; 2],
    api_client_id: Uuid,
    api_keys: [String; 2],
}

async fn seed_accounts(database: &TestDatabase, count: usize) -> Result<Vec<LoadAccount>> {
    let identity = database.identity_service().await?;
    let organizations = OrganizationService::new(identity.clone());
    let api_access = ApiAccessService::new(identity.clone(), organizations.clone());
    let mut accounts = Vec::with_capacity(count);
    for index in 0..count {
        let (actor, credentials) = database.verified_account(&identity).await?;
        let email = identity.get_account(&actor).await?.email;
        let LoginOutcome::Session(revocable) = identity
            .login_with_password(&email, TEST_PASSWORD.into(), TraceContext::default())
            .await?
        else {
            anyhow::bail!("load fixture unexpectedly required MFA");
        };
        let first = organizations
            .create_organization(&actor, format!("Load account {} primary", index + 1))
            .await?;
        let second = organizations
            .create_organization(&actor, format!("Load account {} secondary", index + 1))
            .await?;
        let (commenter, commenter_credentials) = database.verified_account(&identity).await?;
        organizations
            .add_member(&actor, first.id, commenter.account_id())
            .await?;
        let api_client = api_access
            .create_client(
                &actor,
                first.id,
                "Load partner".into(),
                vec![
                    application::api_access::ApiScope::OrganizationsRead,
                    application::api_access::ApiScope::PostsRead,
                ],
                TraceContext::default(),
            )
            .await?;
        api_access
            .set_quota_by_operator(api_client.id, 100_000, 1_000_000, TraceContext::default())
            .await?;
        let primary = api_access
            .issue_key(
                &actor,
                first.id,
                api_client.id,
                "Primary".into(),
                1,
                TraceContext::default(),
            )
            .await?;
        let rotated = api_access
            .issue_key(
                &actor,
                first.id,
                api_client.id,
                "Rotation".into(),
                1,
                TraceContext::default(),
            )
            .await?;
        accounts.push(LoadAccount {
            account_id: actor.account_id(),
            token: credentials.token,
            commenter_token: commenter_credentials.token,
            revocable_token: revocable.token,
            revocable_session_id: revocable.session_id,
            organization_ids: [first.id, second.id],
            api_client_id: api_client.id,
            api_keys: [
                primary.credential.expose().into(),
                rotated.credential.expose().into(),
            ],
        });
    }
    Ok(accounts)
}

fn configure_file_integrations(environment: &mut ProcessEnvironment) -> Result<()> {
    let storage_endpoint = std::env::var("TEST_S3_ENDPOINT")
        .context("load tests require TEST_S3_ENDPOINT; run just load-test-local")?;
    let search_endpoint = std::env::var("TEST_MEILISEARCH_URL")
        .context("load tests require TEST_MEILISEARCH_URL; run just load-test-local")?;
    let mut public_storage_endpoint: url::Url = storage_endpoint.parse()?;
    if !cfg!(target_os = "linux") {
        public_storage_endpoint.set_host(Some("host.docker.internal"))?;
    }
    for endpoint in [&storage_endpoint, &search_endpoint] {
        let url: url::Url = endpoint.parse()?;
        ensure!(
            matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
            "load test file integrations must be local"
        );
    }
    environment.values.extend(
        [
            ("S3_ENDPOINT", storage_endpoint),
            ("S3_PUBLIC_ENDPOINT", public_storage_endpoint.to_string()),
            ("S3_BUCKET", "kit-files".into()),
            ("S3_REGION", "us-east-1".into()),
            ("S3_ACCESS_KEY", "kit-local-access".into()),
            ("S3_SECRET_KEY", "kit-local-development-secret-only".into()),
            ("S3_ALLOW_HTTP", "true".into()),
            ("MEILISEARCH_URL", search_endpoint),
            (
                "MEILISEARCH_API_KEY",
                "kit-local-meilisearch-key-development-only".into(),
            ),
            (
                "MEILISEARCH_INDEX",
                format!("load_files_{}", Uuid::now_v7().simple()),
            ),
            (
                "MEILISEARCH_POSTS_INDEX",
                format!("load_posts_{}", Uuid::now_v7().simple()),
            ),
            (
                "MEILISEARCH_PROFILES_INDEX",
                format!("load_profiles_{}", Uuid::now_v7().simple()),
            ),
            ("MEILISEARCH_ALLOW_HTTP", "true".into()),
            ("WORKER_CONCURRENCY", "4".into()),
            ("WORKER_POLL_MILLISECONDS", "50".into()),
            ("FILE_UPLOAD_GRACE_SECONDS", "30".into()),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value)),
    );
    Ok(())
}

async fn wait_for_file_cleanup(database: &TestDatabase, worker: &mut RunningProcess) -> Result<()> {
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            ensure!(
                worker.child.try_wait()?.is_none(),
                "load test worker exited before file cleanup"
            );
            ensure!(
                job_attempt::Entity::find()
                    .filter(job_attempt::Column::ErrorCode.is_not_null())
                    .count(&database.connection)
                    .await?
                    == 0,
                "load test background work required retries or failed attempts"
            );
            ensure!(
                job::Entity::find()
                    .filter(job::Column::State.eq(jobs::JobState::Failed))
                    .count(&database.connection)
                    .await?
                    == 0,
                "load test left permanently failed background jobs"
            );
            let remaining_files = file::Entity::find()
                .filter(file::Column::State.ne(application::files::FileState::Deleted))
                .count(&database.connection)
                .await?;
            let remaining_jobs = job::Entity::find()
                .filter(
                    job::Column::State.is_in([jobs::JobState::Pending, jobs::JobState::Running]),
                )
                .count(&database.connection)
                .await?;
            if remaining_files == 0 && remaining_jobs == 0 {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("load file deletion and background jobs did not finish")?
}

fn report_failure_diagnostics(process: &RunningProcess, name: &str) -> Result<()> {
    let mut error_counts = BTreeMap::<String, usize>::new();
    let mut server_status_counts = BTreeMap::<u64, usize>::new();
    for line in std::fs::read_to_string(process.log.path())?.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(code) = event["fields"]["error_code"].as_str()
            && code.len() <= 64
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            *error_counts.entry(code.to_owned()).or_default() += 1;
        }
        if let Some(status) = event["span"]["status"].as_u64()
            && status >= 500
        {
            *server_status_counts.entry(status).or_default() += 1;
        }
    }
    eprintln!("{name}: error_codes={error_counts:?}, server_statuses={server_status_counts:?}");
    Ok(())
}

#[tokio::test]
#[ignore = "requires Docker k6, PostgreSQL, Mailpit, SeaweedFS and Meilisearch; run just load-test"]
async fn mixed_workload() -> Result<()> {
    ensure!(
        std::env::var("RUN_K6").as_deref() == Ok("true"),
        "use just load-test"
    );
    let profile = std::env::var("K6_PROFILE").unwrap_or_else(|_| "smoke".into());
    let account_count = match profile.as_str() {
        "smoke" => 2,
        "extended" => 20,
        _ => anyhow::bail!("K6_PROFILE must be smoke or extended"),
    };
    let database = TestDatabase::create().await?;
    let accounts = seed_accounts(&database, account_count).await?;
    let mut environment = ProcessEnvironment::new(&database)?;
    configure_file_integrations(&mut environment)?;
    let settings = kit::config::Settings::from_values(&environment.values)?;
    let application = kit::bootstrap::bootstrap(&settings).await?;
    kit::integrations::initialize(&application).await?;
    application.close().await?;
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let (mut first, _, first_port) = environment.api(&client, true).await?;
    let (mut second, _, second_port) = environment.api(&client, true).await?;
    let mut worker = environment.start(env!("CARGO_BIN_EXE_worker"), None)?;
    let result = async {
        let scripts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/load");
        let mut docker = Command::new("docker");
        docker.args(["run", "--rm"]);
        // Linux host networking reaches the providers' loopback-only published ports.
        let host = if cfg!(target_os = "linux") {
            docker.args(["--network", "host"]);
            "127.0.0.1"
        } else {
            docker.args(["--add-host", "host.docker.internal:host-gateway"]);
            "host.docker.internal"
        };
        let status = docker
            .args([
                "--mount",
                &format!(
                    "type=bind,source={},target=/scripts,readonly",
                    scripts.display()
                ),
                "-e",
                "FIRST_API_URL",
                "-e",
                "SECOND_API_URL",
                "-e",
                "LOAD_ACCOUNTS",
                "-e",
                "K6_PROFILE",
                "grafana/k6:2.2.0",
                "run",
                "--log-output=none",
                "/scripts/mixed.js",
            ])
            .env("FIRST_API_URL", format!("http://{host}:{first_port}"))
            .env("SECOND_API_URL", format!("http://{host}:{second_port}"))
            .env("LOAD_ACCOUNTS", serde_json::to_string(&accounts)?)
            .env("K6_PROFILE", profile)
            .kill_on_drop(true)
            .status()
            .await?;
        ensure!(status.success(), "k6 mixed-workload thresholds failed");
        for account in &accounts {
            let recorded = api_usage::Entity::find()
                .filter(api_usage::Column::ClientId.eq(account.api_client_id))
                .count(&database.connection)
                .await?;
            ensure!(
                recorded >= 4 && recorded.is_multiple_of(4),
                "load workflow admission history is incomplete"
            );
        }
        // This isolated fixture advances the writer horizon after all k6 PUTs finish.
        file::Entity::update_many()
            .col_expr(
                file::Column::UploadExpiresAt,
                Expr::value(chrono::Utc::now() - chrono::Duration::hours(2)),
            )
            .col_expr(
                file::Column::CleanupScheduledAt,
                Expr::value(None::<chrono::DateTime<chrono::Utc>>),
            )
            .filter(file::Column::State.eq(application::files::FileState::Deleting))
            .exec(&database.connection)
            .await?;
        let cleanup = kit::bootstrap::bootstrap(&settings).await?;
        while cleanup.files.reconcile_files().await? > 0 {}
        cleanup.close().await?;
        wait_for_file_cleanup(&database, &mut worker).await
    }
    .await;
    let mut secrets = vec![TEST_PASSWORD];
    for account in &accounts {
        secrets.extend([
            account.token.as_str(),
            account.commenter_token.as_str(),
            account.revocable_token.as_str(),
            account.api_keys[0].as_str(),
            account.api_keys[1].as_str(),
        ]);
    }
    for (name, process) in [
        ("first_api", &mut first),
        ("second_api", &mut second),
        ("worker", &mut worker),
    ] {
        process.stop().await?;
        process.assert_secrets_absent(&secrets)?;
        if result.is_err() {
            report_failure_diagnostics(process, name)?;
        }
    }
    database.finish().await?;
    result
}
