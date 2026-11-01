use crate::{infrastructure, run};
use anyhow::{Context, Result};
use base64::Engine;
use std::{fs, io::Write, path::Path, process::Command};

pub fn initialize() -> Result<()> {
    initialize_at(Path::new("."))
}

fn initialize_at(root: &Path) -> Result<()> {
    let secret = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(auth_core::token::generate_secure_token())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(secret);
    let values = serde_json::json!({"local-v1":encoded});
    let contents = fs::read_to_string(root.join(".envs/.example/local/app.env"))?
        .replace("\r\n", "\n")
        .replace("SECRET_KEYS='{}'", &format!("SECRET_KEYS='{values}'"))
        .replace(
            "RATE_LIMIT_KEY=\n",
            &format!(
                "RATE_LIMIT_KEY={}\n",
                auth_core::token::generate_secure_token()
            ),
        );
    let parsed = dotenvy::from_read_iter(contents.as_bytes())
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    let keys: serde_json::Value = serde_json::from_str(
        parsed
            .get("SECRET_KEYS")
            .context("generated keys missing")?,
    )?;
    anyhow::ensure!(
        keys["local-v1"].as_str().is_some_and(|key| key.len() == 44),
        "invalid generated encryption key"
    );
    fs::create_dir_all(root.join(".envs/.local"))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(".envs/.local/app.env"))
        .context(
            "local app.env already exists or cannot be created; existing settings were not changed",
        )?;
    file.write_all(contents.as_bytes())?;
    println!("Created .envs/.local/app.env with independent local secrets.");
    Ok(())
}

pub fn verify(root: &Path) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let copy = temporary.path().join("renamed-service");
    copy_source(root, &copy)?;
    initialize_at(&copy)?;
    let original_settings = fs::read(copy.join(".envs/.local/app.env"))?;
    anyhow::ensure!(
        initialize_at(&copy).is_err(),
        "initialization replaced existing settings"
    );
    anyhow::ensure!(
        fs::read(copy.join(".envs/.local/app.env"))? == original_settings,
        "existing secrets changed"
    );
    for file in ["Cargo.toml", "Cargo.lock"] {
        let path = copy.join(file);
        let contents = fs::read_to_string(&path)?
            .replace("name = \"hirune-axum-kit\"", "name = \"renamed-service\"");
        fs::write(path, contents)?;
    }
    run(Command::new("cargo")
        .current_dir(&copy)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .args(["test", "--locked", "--workspace"]))?;
    infrastructure::run_tests_at(&copy, &["identity"], false)?;
    println!("Copied and renamed service passed workspace and database tests.");
    Ok(())
}

fn copy_source(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if source.file_name().is_some_and(|name| name == ".envs")
            && name_text != ".example"
            && name_text != "README.md"
        {
            continue;
        }
        if source.file_name().is_some_and(|name| name == "deploy") && name_text.ends_with(".env") {
            continue;
        }
        if matches!(
            name_text.as_ref(),
            ".git" | "target" | ".idea" | ".env" | "logs"
        ) || name_text.starts_with(".env.") && name_text != ".env.example"
        {
            continue;
        }
        if entry.file_type()?.is_symlink() {
            continue;
        }
        let target = destination.join(&name);
        if entry.file_type()?.is_dir() {
            copy_source(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
