use anyhow::{Context, Result, ensure};
use reqwest::{Client, StatusCode};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};
use std::{collections::BTreeMap, net::TcpListener, process::Stdio, time::Duration};
use tempfile::{NamedTempFile, TempDir};
use test_support::{TEST_PASSWORD, TestDatabase};
use tokio::process::{Child, Command};
use uuid::Uuid;

#[path = "process/load.rs"]
mod load;

#[path = "process/api_access.rs"]
mod api_access;
#[path = "process/maintenance.rs"]
mod maintenance;

struct RunningProcess {
    child: Child,
    log: NamedTempFile,
}
impl RunningProcess {
    async fn stop(&mut self) -> Result<()> {
        if self.child.try_wait()?.is_none() {
            self.child.kill().await?;
        }
        Ok(())
    }
    fn assert_secrets_absent(&self, secrets: &[&str]) -> Result<()> {
        let logs = std::fs::read_to_string(self.log.path())?;
        for secret in secrets {
            ensure!(!logs.contains(secret), "a secret was found in process logs");
        }
        Ok(())
    }
}

struct ProcessEnvironment {
    values: BTreeMap<String, String>,
    directory: TempDir,
}
impl ProcessEnvironment {
    fn new(database: &TestDatabase) -> Result<Self> {
        let smtp = std::env::var("TEST_SMTP_PORT")
            .context("TEST_SMTP_PORT required; run just test-process")?;
        Ok(Self {
            directory: tempfile::tempdir()?,
            values: [
                ("APP_ENV", "test".into()),
                ("APP_NAME", "Process Verification".into()),
                ("PUBLIC_URL", "http://localhost:3000/".into()),
                ("ALLOWED_ORIGINS", "http://localhost:3000".into()),
                ("DATABASE_URL", database.url.clone()),
                ("DATABASE_MAX_CONNECTIONS", "6".into()),
                ("ACTIVE_SECRET_KEY", "test-v1".into()),
                (
                    "SECRET_KEYS",
                    r#"{"test-v1":"ERERERERERERERERERERERERERERERERERERERERERE="}"#.into(),
                ),
                (
                    "RATE_LIMIT_KEY",
                    "test-only-rate-limit-key-at-least-32-bytes".into(),
                ),
                ("SMTP_HOST", "127.0.0.1".into()),
                ("SMTP_PORT", smtp),
                ("SMTP_STARTTLS", "false".into()),
                ("MAIL_FROM", "Test <noreply@example.test>".into()),
                ("PASSWORD_CONCURRENCY", "2".into()),
                ("WORKER_CONCURRENCY", "2".into()),
                ("JOB_LEASE_SECONDS", "10".into()),
                ("JOB_EXECUTION_SECONDS", "30".into()),
                ("WORKER_DRAIN_SECONDS", "12".into()),
                (
                    "RUST_LOG",
                    "info,sqlx=off,sea_orm=off,reqwest=off,hyper=off".into(),
                ),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
        })
    }
    fn start(&self, binary: &str, bind: Option<&str>) -> Result<RunningProcess> {
        self.start_with_arguments(binary, &[], bind)
    }
    fn start_with_arguments(
        &self,
        binary: &str,
        arguments: &[&str],
        bind: Option<&str>,
    ) -> Result<RunningProcess> {
        let log = NamedTempFile::new()?;
        let mut command = Command::new(binary);
        command
            .args(arguments)
            .current_dir(self.directory.path())
            .envs(&self.values)
            .env_remove("SENTRY_DSN")
            .env_remove("OIDC_PROVIDERS")
            .env_remove("OIDC_PROVIDER")
            .env_remove("OIDC_ISSUER")
            .env_remove("OIDC_CLIENT_ID")
            .env_remove("OIDC_CLIENT_SECRET")
            .env_remove("OIDC_REDIRECT_URL")
            .env_remove("SMTP_USERNAME")
            .env_remove("SMTP_PASSWORD")
            .env_remove("SMTP_CA_CERTIFICATE")
            .env_remove("TRUSTED_PROXY_IPS")
            .env_remove("INTERNAL_PROXY_SECRET")
            .kill_on_drop(true)
            .stdout(Stdio::from(log.reopen()?))
            .stderr(Stdio::from(log.reopen()?));
        if let Some(bind) = bind {
            command.env("BIND_ADDRESS", bind);
        }
        Ok(RunningProcess {
            child: command.spawn()?,
            log,
        })
    }
    async fn api(&self, client: &Client, external: bool) -> Result<(RunningProcess, String, u16)> {
        let port = TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port();
        let mut process = self.start(
            env!("CARGO_BIN_EXE_api"),
            Some(&format!(
                "{}:{port}",
                if external { "0.0.0.0" } else { "127.0.0.1" }
            )),
        )?;
        let url = format!("http://127.0.0.1:{port}");
        tokio::time::timeout(Duration::from_secs(40), async {
            loop {
                ensure!(
                    process.child.try_wait()?.is_none(),
                    "API process exited before readiness"
                );
                if client
                    .get(format!("{url}/health/ready"))
                    .send()
                    .await
                    .is_ok_and(|response| response.status() == StatusCode::NO_CONTENT)
                {
                    return Ok::<(), anyhow::Error>(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .context("API readiness deadline elapsed")??;
        Ok((process, url, port))
    }
}

async fn verification_token(client: &Client, email: &str, subject: &str) -> Result<String> {
    let detail = received_mail(client, email, subject).await?;
    let text = detail["Text"]
        .as_str()
        .context("plain text mail body missing")?;
    let token = text
        .split("#token=")
        .nth(1)
        .context("mail action token missing")?
        .chars()
        .take(43)
        .collect::<String>();
    ensure!(token.len() == 43, "invalid action token length");
    Ok(token)
}

async fn received_mail(client: &Client, email: &str, subject: &str) -> Result<Value> {
    let mailpit = std::env::var("TEST_MAILPIT_URL").context("TEST_MAILPIT_URL required")?;
    tokio::time::timeout(Duration::from_secs(35), async {
        loop {
            let response: Value = client
                .get(format!("{mailpit}/api/v1/search"))
                .query(&[("query", format!("to:{email}"))])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            if let Some(messages) = response["messages"].as_array() {
                for message in messages {
                    if !message["Subject"]
                        .as_str()
                        .unwrap_or_default()
                        .contains(subject)
                    {
                        continue;
                    }
                    let id = message["ID"]
                        .as_str()
                        .context("Mailpit message id missing")?;
                    let detail: Value = client
                        .get(format!("{mailpit}/api/v1/message/{id}"))
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;
                    ensure!(
                        detail["Text"].as_str().is_some_and(|text| !text.is_empty()),
                        "plain text mail body missing"
                    );
                    ensure!(
                        detail["HTML"]
                            .as_str()
                            .is_some_and(|html| html.contains("<html")),
                        "MJML HTML missing"
                    );
                    return Ok::<_, anyhow::Error>(detail);
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .context("worker did not deliver the expected email")?
}

#[tokio::test]
#[ignore = "requires PostgreSQL and Mailpit; run just test-process"]
async fn signup_mail_login_cross_process_revocation_and_worker_restart() -> Result<()> {
    let database = TestDatabase::create().await?;
    let environment = ProcessEnvironment::new(&database)?;
    let client = Client::builder().timeout(Duration::from_secs(20)).build()?;
    let (mut first, first_url, _) = environment.api(&client, false).await?;
    let (mut second, second_url, _) = environment.api(&client, false).await?;
    let email = format!("process-{}@example.test", Uuid::now_v7());
    let response = client
        .post(format!("{first_url}/api/v1/auth/signup"))
        .json(&json!({"email":email,"password":TEST_PASSWORD}))
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let request_id = response.headers()["x-request-id"].to_str()?.to_owned();
    let mut worker = environment.start(env!("CARGO_BIN_EXE_worker"), None)?;
    let verification = verification_token(&client, &email, "Verify your email").await?;
    assert_eq!(
        client
            .post(format!("{second_url}/api/v1/auth/verification/confirm"))
            .json(&json!({"token":verification}))
            .send()
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    let welcome = received_mail(&client, &email, "Welcome").await?;
    ensure!(
        !welcome["Text"].as_str().unwrap().contains("#token="),
        "welcome mail must not carry an action token"
    );
    let login: Value = client
        .post(format!("{first_url}/api/v1/auth/login"))
        .json(&json!({"email":email,"password":TEST_PASSWORD,"transport":"bearer"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let token = login["token"]
        .as_str()
        .context("bearer login token missing")?;
    assert_eq!(
        client
            .get(format!("{second_url}/api/v1/account"))
            .bearer_auth(token)
            .send()
            .await?
            .status(),
        StatusCode::OK
    );
    let rotated: Value = client
        .post(format!("{first_url}/api/v1/sessions/current/rotate"))
        .bearer_auth(token)
        .json(&json!({"transport":"bearer"}))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let rotated_token = rotated["token"].as_str().context("rotated token missing")?;
    assert_eq!(
        client
            .get(format!("{second_url}/api/v1/account"))
            .bearer_auth(token)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    worker.stop().await?;
    assert_eq!(
        client
            .post(format!("{second_url}/api/v1/auth/password/reset"))
            .json(&json!({"email":email}))
            .send()
            .await?
            .status(),
        StatusCode::ACCEPTED
    );
    let mut replacement = environment.start(env!("CARGO_BIN_EXE_worker"), None)?;
    let reset_token = verification_token(&client, &email, "Reset your password").await?;
    let next_password = "another strong process passphrase";
    assert_eq!(
        client
            .post(format!("{first_url}/api/v1/auth/password/reset/confirm"))
            .json(&json!({"token":reset_token,"new_password":next_password}))
            .send()
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        client
            .get(format!("{second_url}/api/v1/account"))
            .bearer_auth(rotated_token)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    use application::identity::entities::mail_delivery;
    let changed = received_mail(&client, &email, "Your password was changed").await?;
    ensure!(
        !changed["Text"].as_str().unwrap().contains(next_password),
        "notification included a password"
    );
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if mail_delivery::Entity::find()
                .filter(mail_delivery::Column::Ciphertext.is_not_null())
                .count(&database.connection)
                .await?
                == 0
            {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await??;
    tokio::time::sleep(Duration::from_millis(300)).await;
    for process in [&mut first, &mut second, &mut worker, &mut replacement] {
        process.stop().await?;
        process.assert_secrets_absent(&[
            TEST_PASSWORD,
            next_password,
            &verification,
            &reset_token,
            token,
            rotated_token,
        ])?;
    }
    ensure!(
        std::fs::read_to_string(worker.log.path())?.contains(&request_id),
        "request correlation did not reach worker logs"
    );
    database.finish().await
}
