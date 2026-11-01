use crate::run;
use anyhow::{Result, ensure};
use std::{net::TcpListener, path::Path, process::Command};

pub fn run_tests(targets: &[&str]) -> Result<()> {
    run_tests_at(Path::new("."), targets, false)
}
pub fn run_load_test() -> Result<()> {
    run_tests_at(Path::new("."), &["process"], true)
}

pub fn run_tests_at(directory: &Path, targets: &[&str], load: bool) -> Result<()> {
    let requires_mail = targets.contains(&"process");
    let external_database = std::env::var("TEST_DATABASE_URL").ok();
    let infrastructure = if external_database.is_none() {
        Some(Infrastructure::start(
            directory,
            targets.contains(&"files") || targets.contains(&"content") || load,
        )?)
    } else {
        None
    };
    let mut command = Command::new("cargo");
    command.current_dir(directory).args(["test", "--locked"]);
    command.env(
        "CARGO_TARGET_DIR",
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("target"),
    );
    for target in targets {
        command.args(["--test", target]);
    }
    let job_load = targets.contains(&"job_load");
    let parallel_test_cases = if targets.contains(&"jobs") || job_load {
        1
    } else {
        2
    };
    if job_load {
        match std::env::var("JOB_LOAD_RELEASE").as_deref() {
            Ok("true") => {
                command.arg("--release");
            }
            Err(std::env::VarError::NotPresent) | Ok("false") => {}
            _ => anyhow::bail!("JOB_LOAD_RELEASE must be true or false"),
        }
    }
    command
        .arg("--")
        .arg("--ignored")
        .arg(format!("--test-threads={parallel_test_cases}"));
    if load {
        command
            .args(["load::mixed_workload", "--nocapture"])
            .env("RUN_K6", "true");
    } else {
        command.args(["--skip", "load::mixed_workload"]);
    }
    if job_load {
        command.arg("--nocapture");
    }
    if let Some(infrastructure) = &infrastructure {
        command
            .env(
                "TEST_DATABASE_URL",
                format!(
                    "postgres://kit:local-development-only@localhost:{}/kit",
                    infrastructure.postgres_port
                ),
            )
            .env("TEST_SMTP_PORT", infrastructure.smtp_port.to_string())
            .env(
                "TEST_MAILPIT_URL",
                format!("http://localhost:{}", infrastructure.mailpit_port),
            );
        if infrastructure.files {
            command
                .env(
                    "TEST_S3_ENDPOINT",
                    format!("http://localhost:{}", infrastructure.s3_port),
                )
                .env(
                    "TEST_MEILISEARCH_URL",
                    format!("http://localhost:{}", infrastructure.meilisearch_port),
                );
        }
    } else if requires_mail {
        ensure!(
            std::env::var_os("TEST_SMTP_PORT").is_some()
                && std::env::var_os("TEST_MAILPIT_URL").is_some(),
            "external process tests require TEST_SMTP_PORT and TEST_MAILPIT_URL"
        );
    }
    if infrastructure.is_none() && load {
        ensure!(
            std::env::var_os("TEST_S3_ENDPOINT").is_some()
                && std::env::var_os("TEST_MEILISEARCH_URL").is_some(),
            "external load tests require TEST_S3_ENDPOINT and TEST_MEILISEARCH_URL"
        );
    }
    let result = run(&mut command);
    if result.is_err()
        && let Some(infrastructure) = &infrastructure
        && infrastructure.files
    {
        let _ = infrastructure
            .command()
            .args([
                "logs",
                "--no-color",
                "--tail",
                "100",
                "seaweedfs",
                "meilisearch",
            ])
            .status();
    }
    result
}

struct Infrastructure {
    directory: std::path::PathBuf,
    project: String,
    postgres_port: u16,
    smtp_port: u16,
    mailpit_port: u16,
    s3_port: u16,
    meilisearch_port: u16,
    files: bool,
}
impl Infrastructure {
    fn start(directory: &Path, files: bool) -> Result<Self> {
        let infrastructure = Self {
            directory: directory.to_path_buf(),
            project: format!("kit-check-{}", uuid::Uuid::now_v7().simple()),
            postgres_port: available_port()?,
            smtp_port: available_port()?,
            mailpit_port: available_port()?,
            s3_port: available_port()?,
            meilisearch_port: available_port()?,
            files,
        };
        run(infrastructure
            .command()
            .args(["up", "-d", "--wait", "--wait-timeout", "90"]))?;
        Ok(infrastructure)
    }
    fn command(&self) -> Command {
        let mut command = Command::new("docker");
        command
            .current_dir(&self.directory)
            .args([
                "compose",
                "-f",
                "compose/compose.local.yml",
                "-p",
                &self.project,
            ])
            .env("POSTGRES_PORT", self.postgres_port.to_string())
            .env("SMTP_PORT", self.smtp_port.to_string())
            .env("MAILPIT_PORT", self.mailpit_port.to_string());
        command
            .env("S3_PORT", self.s3_port.to_string())
            .env("MEILISEARCH_PORT", self.meilisearch_port.to_string());
        if self.files {
            command.args(["--profile", "files", "--profile", "search"]);
        }
        command
    }
}
impl Drop for Infrastructure {
    fn drop(&mut self) {
        if !self
            .command()
            .args(["down", "--volumes", "--remove-orphans"])
            .status()
            .is_ok_and(|status| status.success())
        {
            eprintln!(
                "could not remove test infrastructure; inspect Docker Compose project {}",
                self.project
            );
        }
    }
}
fn available_port() -> Result<u16> {
    Ok(TcpListener::bind(("127.0.0.1", 0))?.local_addr()?.port())
}
