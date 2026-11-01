mod checks;
mod data_services;
mod deployment;
mod infrastructure;
mod kubernetes;
mod kubernetes_autoscaling;
mod kubernetes_environment;
mod kubernetes_jobs;
mod kubernetes_maintenance;
mod kubernetes_test;
mod production;
mod template;

use anyhow::{Context, Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("workspace root missing")?;
    std::env::set_current_dir(root)?;
    match std::env::args().nth(1).as_deref() {
        Some("init") => template::initialize(),
        Some("check") => checks::check(),
        Some("openapi") => checks::openapi(false),
        Some("test-db") => infrastructure::run_tests(&["identity", "jobs", "schema_consistency"]),
        Some("test-job-load") => infrastructure::run_tests(&["job_load"]),
        Some("test-api") => {
            infrastructure::run_tests(&["api", "oidc", "api_access", "human_verification"])
        }
        Some("test-process") => infrastructure::run_tests(&["process"]),
        Some("test-files") => infrastructure::run_tests(&["files"]),
        Some("test-content") => infrastructure::run_tests(&["content"]),
        Some("test-deployment") => deployment::verify(),
        Some("test-production") => production::verify(),
        Some("deploy-k3s") => kubernetes::from_arguments(),
        Some("data-services") => data_services::from_arguments(),
        Some("test-k3s") => kubernetes_test::verify(),
        Some("test-data-services") => {
            crate::run(Command::new("python3").arg("compose/data-services/verify.py"))
        }
        Some("load-test") => kubernetes_test::verify(),
        Some("load-test-local") => infrastructure::run_load_test(),
        Some("template-check") => template::verify(root),
        _ => anyhow::bail!(
            "usage: cargo xtask <init|check|openapi|test-db|test-job-load|test-api|test-process|test-files|test-content|test-deployment|test-production|test-data-services|data-services <action> <dev|prod>|deploy-k3s <action> <dev|prod>|test-k3s|load-test|load-test-local|template-check>"
        ),
    }
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .context("could not start development command")?;
    ensure!(status.success(), "development command failed: {status}");
    Ok(())
}

fn rust_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(rust_files(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}
