use crate::{run, rust_files};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path, process::Command};

pub fn check() -> Result<()> {
    run(Command::new("cargo").args(["fmt", "--all", "--", "--check"]))?;
    check_boundaries()?;
    check_document_links(Path::new("."))?;
    run(Command::new("cargo").args([
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]))?;
    run(Command::new("cargo").args(["test", "--locked", "--workspace"]))?;
    openapi(true)
}

pub fn openapi(check: bool) -> Result<()> {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--locked", "--bin", "openapi"])
        .output()?;
    ensure!(
        output.status.success(),
        "OpenAPI generation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let generated: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let path = "docs/openapi.json";
    if check {
        let saved: serde_json::Value = serde_json::from_slice(
            &fs::read(path).context("run just openapi and commit its output")?,
        )?;
        ensure!(
            saved == generated,
            "OpenAPI changed; run just openapi and review the contract"
        );
    } else {
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&generated)?),
        )?;
    }
    Ok(())
}

fn check_boundaries() -> Result<()> {
    let output = Command::new("cargo")
        .args(["metadata", "--locked", "--no-deps", "--format-version", "1"])
        .output()?;
    ensure!(output.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    for package in metadata["packages"]
        .as_array()
        .context("packages missing")?
    {
        let name = package["name"].as_str().context("package name missing")?;
        for dependency in package["dependencies"]
            .as_array()
            .context("dependencies missing")?
        {
            if dependency["kind"] == "dev" {
                continue;
            }
            let dependency = dependency["name"]
                .as_str()
                .context("dependency name missing")?;
            if matches!(name, "application" | "jobs" | "mail" | "auth-core") {
                ensure!(
                    !matches!(
                        dependency,
                        "axum" | "tower-http" | "kit" | "test-support" | "migration"
                    ),
                    "{name} must not depend on {dependency}"
                );
            }
            if matches!(name, "jobs" | "mail" | "auth-core") {
                ensure!(
                    dependency != "application",
                    "infrastructure must not depend on application services"
                );
            }
        }
    }
    for path in rust_files(Path::new("src/http"))? {
        let source = fs::read_to_string(&path)?;
        ensure!(
            !source.contains("sea_orm") && !source.contains("::entities"),
            "HTTP persistence boundary violated in {}",
            path.display()
        );
    }
    Ok(())
}

fn check_document_links(directory: &Path) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            if directory != Path::new(".")
                || matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("docs" | "memory" | ".envs" | "compose" | "deploy")
                )
            {
                check_document_links(&path)?;
            }
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        let content = fs::read_to_string(&path)?;
        for link in content
            .split("](")
            .skip(1)
            .filter_map(|rest| rest.split_once(')').map(|(link, _)| link))
        {
            let link = link.split('#').next().unwrap_or_default();
            if link.is_empty() || link.contains("://") || link.starts_with("mailto:") {
                continue;
            }
            ensure!(
                path.parent()
                    .context("document parent missing")?
                    .join(link)
                    .exists(),
                "broken local link in {}: {link}",
                path.display()
            );
        }
    }
    Ok(())
}
